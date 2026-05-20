//! Store-layer CRUD for the four calendar tables, plus per-account
//! Keychain blob helpers. Constructed with a `Store` clone, same pattern
//! as `Settings` and `Reference`.

use crate::calendar::types::{
    AttendeeStatus, Calendar, CalendarAccount, CalendarEvent, EventDecision, ProviderKind,
};
use crate::store::Store;
use crate::{time, Result};

/// Row shape returned by the `calendar_accounts` SELECT queries.
type AccountRow = (String, String, String, String, Option<String>, i64, String);

/// Row shape returned by the `calendars` SELECT queries.
type CalendarRow = (String, String, String, Option<String>, i64);

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
            "SELECT id, account_id, name, color, included
             FROM calendars WHERE account_id = ? ORDER BY name",
        )
        .bind(account_id)
        .fetch_all(self.store.pool())
        .await?;
        Ok(rows
            .into_iter()
            .map(|(id, account_id, name, color, included)| Calendar {
                id,
                account_id,
                name,
                color,
                included: included != 0,
            })
            .collect())
    }

    pub async fn set_calendar_included(&self, calendar_id: &str, included: bool) -> Result<()> {
        sqlx::query("UPDATE calendars SET included = ? WHERE id = ?")
            .bind(if included { 1 } else { 0 })
            .bind(calendar_id)
            .execute(self.store.pool())
            .await?;
        Ok(())
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

// Suppress dead-code warnings on imports the later tasks will actually use.
#[allow(dead_code)]
fn _phantom_imports(
    _: AttendeeStatus,
    _: Calendar,
    _: CalendarEvent,
    _: EventDecision,
    _: &dyn FnOnce() -> String,
) {
    let _ = time::now_utc;
}
