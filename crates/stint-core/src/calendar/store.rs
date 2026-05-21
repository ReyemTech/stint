//! Store-layer CRUD for the four calendar tables, plus per-account
//! Keychain blob helpers. Constructed with a `Store` clone, same pattern
//! as `Settings` and `Reference`.

use crate::calendar::types::{
    AttendeeStatus, Calendar, CalendarAccount, CalendarEvent, EventDecision, ProviderKind,
};
use crate::config::secrets::Secrets;
use crate::oauth::tokens::TokenSet;
use crate::store::Store;
use crate::{time, Result};
use serde::{Deserialize, Serialize};

/// Row shape returned by the `calendar_accounts` SELECT queries.
type AccountRow = (String, String, String, String, Option<String>, i64, String);

/// Row shape returned by the `calendars` SELECT queries.
type CalendarRow = (String, String, String, Option<String>, i64, Option<String>);

/// Row shape returned by the `calendar_events` SELECT queries.
type EventRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    i64,
    Option<String>,
    Option<String>,
    String,
);

pub struct CalendarStore {
    store: Store,
}

impl CalendarStore {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    pub async fn add_account(&self, a: &CalendarAccount) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO calendar_accounts
               (id, provider, display_name, identifier, caldav_url, enabled, created_at)
               VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&a.id)
        .bind(provider_wire(a.provider))
        .bind(&a.display_name)
        .bind(&a.identifier)
        .bind(&a.caldav_url)
        .bind(if a.enabled { 1 } else { 0 })
        .bind(&a.created_at)
        .execute(self.store.pool())
        .await?;
        Ok(())
    }

    pub async fn get_account(&self, id: &str) -> Result<Option<CalendarAccount>> {
        let row: Option<AccountRow> = sqlx::query_as(
            "SELECT id, provider, display_name, identifier, caldav_url, enabled, created_at
                 FROM calendar_accounts WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(self.store.pool())
        .await?;
        Ok(row.map(account_from_row))
    }

    pub async fn list_accounts(&self) -> Result<Vec<CalendarAccount>> {
        let rows: Vec<AccountRow> = sqlx::query_as(
            "SELECT id, provider, display_name, identifier, caldav_url, enabled, created_at
                 FROM calendar_accounts ORDER BY created_at",
        )
        .fetch_all(self.store.pool())
        .await?;
        Ok(rows.into_iter().map(account_from_row).collect())
    }

    pub async fn delete_account(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM calendar_accounts WHERE id = ?")
            .bind(id)
            .execute(self.store.pool())
            .await?;
        Ok(())
    }

    pub async fn set_account_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        sqlx::query("UPDATE calendar_accounts SET enabled = ? WHERE id = ?")
            .bind(if enabled { 1 } else { 0 })
            .bind(id)
            .execute(self.store.pool())
            .await?;
        Ok(())
    }

    /// Upserts a provider-returned set of calendars for one account. The
    /// `included` field on the input is ignored — locality of the include
    /// flag is preserved by an `ON CONFLICT` that doesn't touch it. New
    /// rows default `included = 1` per the schema.
    pub async fn upsert_calendars(&self, account_id: &str, calendars: &[Calendar]) -> Result<()> {
        let mut tx = self.store.pool().begin().await?;
        for c in calendars {
            sqlx::query(
                r#"INSERT INTO calendars (id, account_id, name, color, included)
                   VALUES (?, ?, ?, ?, 1)
                   ON CONFLICT(id) DO UPDATE SET
                     name = excluded.name,
                     color = excluded.color
                     -- intentionally not touching included
                "#,
            )
            .bind(&c.id)
            .bind(account_id)
            .bind(&c.name)
            .bind(&c.color)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_calendars(&self, account_id: &str) -> Result<Vec<Calendar>> {
        let rows: Vec<CalendarRow> = sqlx::query_as(
            "SELECT id, account_id, name, color, included, default_project_id
             FROM calendars WHERE account_id = ? ORDER BY name",
        )
        .bind(account_id)
        .fetch_all(self.store.pool())
        .await?;
        Ok(rows.into_iter().map(calendar_from_row).collect())
    }

    pub async fn set_calendar_included(&self, calendar_id: &str, included: bool) -> Result<()> {
        sqlx::query("UPDATE calendars SET included = ? WHERE id = ?")
            .bind(if included { 1 } else { 0 })
            .bind(calendar_id)
            .execute(self.store.pool())
            .await?;
        Ok(())
    }

    pub async fn set_default_project(
        &self,
        calendar_id: &str,
        project_id: Option<&str>,
    ) -> Result<()> {
        sqlx::query("UPDATE calendars SET default_project_id = ? WHERE id = ?")
            .bind(project_id)
            .bind(calendar_id)
            .execute(self.store.pool())
            .await?;
        Ok(())
    }

    pub async fn upsert_events(&self, events: &[CalendarEvent]) -> Result<()> {
        let mut tx = self.store.pool().begin().await?;
        for e in events {
            sqlx::query(
                r#"INSERT INTO calendar_events
                   (id, account_id, calendar_id, title, start_at, end_at,
                    is_all_day, attendee_status, recurring_root, fetched_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                   ON CONFLICT(account_id, id, start_at) DO UPDATE SET
                     calendar_id = excluded.calendar_id,
                     title = excluded.title,
                     end_at = excluded.end_at,
                     is_all_day = excluded.is_all_day,
                     attendee_status = excluded.attendee_status,
                     recurring_root = excluded.recurring_root,
                     fetched_at = excluded.fetched_at"#,
            )
            .bind(&e.id)
            .bind(&e.account_id)
            .bind(&e.calendar_id)
            .bind(&e.title)
            .bind(&e.start_at)
            .bind(&e.end_at)
            .bind(if e.is_all_day { 1i64 } else { 0i64 })
            .bind(e.attendee_status.map(|s| s.as_wire().to_string()))
            .bind(&e.recurring_root)
            .bind(&e.fetched_at)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Range is half-open `[from, to)` on `start_at`. Joins against
    /// `calendars` so events on excluded calendars are filtered out.
    pub async fn list_events_in_range(
        &self,
        account_id: &str,
        from: &str,
        to: &str,
    ) -> Result<Vec<CalendarEvent>> {
        let rows: Vec<EventRow> = sqlx::query_as(
            r#"SELECT e.id, e.account_id, e.calendar_id, e.title, e.start_at, e.end_at,
                       e.is_all_day, e.attendee_status, e.recurring_root, e.fetched_at
                 FROM calendar_events e
                 JOIN calendars c ON c.id = e.calendar_id
                WHERE e.account_id = ?
                  AND c.included = 1
                  AND e.start_at >= ?
                  AND e.start_at < ?
                ORDER BY e.start_at"#,
        )
        .bind(account_id)
        .bind(from)
        .bind(to)
        .fetch_all(self.store.pool())
        .await?;

        Ok(rows.into_iter().map(event_from_row).collect())
    }

    pub async fn record_decision(
        &self,
        account_id: &str,
        event_id: &str,
        event_start: &str,
        decision: &EventDecision,
    ) -> Result<()> {
        let now = time::now_utc();
        sqlx::query(
            r#"INSERT INTO event_decisions
               (account_id, event_id, event_start, decision, linked_local_uuid, decided_at)
               VALUES (?, ?, ?, ?, ?, ?)
               ON CONFLICT(account_id, event_id, event_start) DO UPDATE SET
                 decision = excluded.decision,
                 linked_local_uuid = excluded.linked_local_uuid,
                 decided_at = excluded.decided_at"#,
        )
        .bind(account_id)
        .bind(event_id)
        .bind(event_start)
        .bind(decision.as_wire())
        .bind(decision.linked_local_uuid())
        .bind(&now)
        .execute(self.store.pool())
        .await?;
        Ok(())
    }

    pub async fn clear_decision(
        &self,
        account_id: &str,
        event_id: &str,
        event_start: &str,
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM event_decisions
             WHERE account_id = ? AND event_id = ? AND event_start = ?",
        )
        .bind(account_id)
        .bind(event_id)
        .bind(event_start)
        .execute(self.store.pool())
        .await?;
        Ok(())
    }

    pub async fn get_decision(
        &self,
        account_id: &str,
        event_id: &str,
        event_start: &str,
    ) -> Result<Option<EventDecision>> {
        let row: Option<(String, Option<String>)> = sqlx::query_as(
            "SELECT decision, linked_local_uuid FROM event_decisions
             WHERE account_id = ? AND event_id = ? AND event_start = ?",
        )
        .bind(account_id)
        .bind(event_id)
        .bind(event_start)
        .fetch_optional(self.store.pool())
        .await?;
        Ok(row.and_then(|(wire, uuid)| EventDecision::decoded(&wire, uuid)))
    }

    /// Returns `(event_id, event_start, decision)` triples for decisions
    /// whose `event_start` falls in `[from, to)`. The event-id form lets
    /// the caller index decisions against an event list cheaply.
    pub async fn list_decisions_in_range(
        &self,
        account_id: &str,
        from: &str,
        to: &str,
    ) -> Result<Vec<(String, String, EventDecision)>> {
        let rows: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
            "SELECT event_id, event_start, decision, linked_local_uuid
             FROM event_decisions
             WHERE account_id = ? AND event_start >= ? AND event_start < ?",
        )
        .bind(account_id)
        .bind(from)
        .bind(to)
        .fetch_all(self.store.pool())
        .await?;
        Ok(rows
            .into_iter()
            .filter_map(|(event_id, event_start, wire, uuid)| {
                EventDecision::decoded(&wire, uuid).map(|d| (event_id, event_start, d))
            })
            .collect())
    }
}

fn provider_wire(p: ProviderKind) -> &'static str {
    match p {
        ProviderKind::Google => "google",
    }
}

fn provider_from_wire(s: &str) -> ProviderKind {
    match s {
        "google" => ProviderKind::Google,
        // Phase 3c/d will extend; for now an unknown value falls back to Google
        // rather than panic — the column is constrained by what we wrote.
        _ => ProviderKind::Google,
    }
}

fn account_from_row(
    (id, provider, display_name, identifier, caldav_url, enabled, created_at): AccountRow,
) -> CalendarAccount {
    CalendarAccount {
        id,
        provider: provider_from_wire(&provider),
        display_name,
        identifier,
        caldav_url,
        enabled: enabled != 0,
        created_at,
    }
}

fn calendar_from_row(
    (id, account_id, name, color, included, default_project_id): CalendarRow,
) -> Calendar {
    Calendar {
        id,
        account_id,
        name,
        color,
        included: included != 0,
        default_project_id,
    }
}

fn event_from_row(
    (
        id,
        account_id,
        calendar_id,
        title,
        start_at,
        end_at,
        is_all_day,
        attendee_status,
        recurring_root,
        fetched_at,
    ): EventRow,
) -> CalendarEvent {
    CalendarEvent {
        id,
        account_id,
        calendar_id,
        title,
        start_at,
        end_at,
        is_all_day: is_all_day != 0,
        attendee_status: attendee_status
            .as_deref()
            .and_then(AttendeeStatus::from_wire),
        recurring_root,
        fetched_at,
    }
}

// ── Per-account Keychain blob helpers ────────────────────────────────────────

/// Per-account OAuth credentials stored in Keychain as one JSON blob.
/// Same shape as the Solidtime OAuth blob (3a) for consistency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarOAuthBlob {
    pub client_id: String,
    /// Some for Google (Desktop client secret); None for providers that
    /// don't issue one. Serialized via `#[serde(default)]` so blobs
    /// written by earlier Task 11 tests don't fail to deserialize.
    #[serde(default)]
    pub client_secret: Option<String>,
    pub tokens: TokenSet,
}

fn calendar_blob_key(account_uuid: &str) -> String {
    format!("calendar.{account_uuid}")
}

pub fn calendar_blob_load(
    secrets: &Secrets,
    account_uuid: &str,
) -> crate::Result<Option<CalendarOAuthBlob>> {
    let Some(raw) = secrets.get(&calendar_blob_key(account_uuid))? else {
        return Ok(None);
    };
    let blob: CalendarOAuthBlob = serde_json::from_str(&raw).map_err(|e| {
        crate::Error::OAuthServer(format!(
            "Calendar Keychain blob malformed for {account_uuid}: {e}"
        ))
    })?;
    Ok(Some(blob))
}

pub fn calendar_blob_save(
    secrets: &Secrets,
    account_uuid: &str,
    blob: &CalendarOAuthBlob,
) -> crate::Result<()> {
    let raw = serde_json::to_string(blob).expect("CalendarOAuthBlob is JSON-serializable");
    secrets.set(&calendar_blob_key(account_uuid), &raw)
}

pub fn calendar_blob_delete(secrets: &Secrets, account_uuid: &str) -> crate::Result<()> {
    secrets.delete(&calendar_blob_key(account_uuid))
}
