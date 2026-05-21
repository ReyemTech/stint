//! Tauri commands for calendar features.
//!
//! Thin wrappers around stint-core::calendar. Auth material is loaded
//! from Keychain per account; an `OAuthTokenProvider` is constructed
//! on demand so refresh tokens rotate transparently.

use crate::app_state::AppState;
use crate::commands::{store, AppError};
use serde::Serialize;
use std::time::Duration;
use stint_core::calendar::google::client::GoogleClient;
use stint_core::calendar::google::config::{google_oauth_config, is_configured};
use stint_core::calendar::store::{
    calendar_blob_delete, calendar_blob_load, calendar_blob_save, CalendarOAuthBlob, CalendarStore,
};
use stint_core::calendar::sync::{refresh_account, Ranges};
use stint_core::calendar::types::{
    Calendar, CalendarAccount, CalendarEvent, EventDecision, ProviderKind,
};
use stint_core::config::secrets::Secrets;
use stint_core::ids;
use stint_core::oauth::client::OAuthClient;
use stint_core::solidtime::auth::login_interactive;
use stint_core::store::entries::{Entries, NewCompletedEntry};
use stint_core::store::queue::{Queue, QueueOp};
use stint_core::time;
use tauri::{Emitter, Runtime, State};
use tokio::sync::RwLock;

pub const EVENT_CALENDAR_CHANGED: &str = "calendar:changed";

#[derive(Serialize)]
pub struct EventWithDecision {
    #[serde(flatten)]
    pub event: CalendarEvent,
    pub decision: Option<String>,
    pub linked_local_uuid: Option<String>,
}

#[derive(Serialize)]
pub struct CalendarOAuthStatus {
    pub signed_in: bool,
    pub scope: Option<String>,
}

#[tauri::command]
pub async fn calendar_list_accounts(
    state: State<'_, RwLock<AppState>>,
) -> Result<Vec<CalendarAccount>, AppError> {
    let store = store(&state).await;
    let cs = CalendarStore::new((*store).clone());
    Ok(cs.list_accounts().await?)
}

#[tauri::command]
pub async fn calendar_oauth_status(account_id: String) -> Result<CalendarOAuthStatus, AppError> {
    let secrets = Secrets::default();
    let blob = calendar_blob_load(&secrets, &account_id)?;
    Ok(CalendarOAuthStatus {
        signed_in: blob.is_some(),
        scope: blob.and_then(|b| b.tokens.scope),
    })
}

#[tauri::command]
pub async fn calendar_add_google<R: Runtime>(
    state: State<'_, RwLock<AppState>>,
    app: tauri::AppHandle<R>,
) -> Result<CalendarAccount, AppError> {
    if !is_configured() {
        return Err(AppError::msg(
            "Google OAuth credentials are not configured in this build. \
             Set STINT_GOOGLE_CLIENT_ID and STINT_GOOGLE_CLIENT_SECRET at build time.",
        ));
    }

    let store = store(&state).await;
    let cs = CalendarStore::new((*store).clone());
    let secrets = Secrets::default();

    // 1) Run the OAuth PKCE flow against accounts.google.com.
    let cfg = google_oauth_config();
    let client_id = cfg.client_id.clone();
    let cfg_client_secret = cfg.client_secret.clone();
    let oauth_client = OAuthClient::new(cfg);
    let tokens = login_interactive(&oauth_client, Duration::from_secs(300), "Google", |url| {
        if let Err(e) = open_url(&url) {
            tracing::warn!("could not open browser: {e}; paste manually: {url}");
        }
    })
    .await?;

    // 2) Insert a placeholder account so we have a UUID to key the blob.
    let account_uuid = ids::new_local_uuid();
    calendar_blob_save(
        &secrets,
        &account_uuid,
        &CalendarOAuthBlob {
            client_id: client_id.clone(),
            client_secret: cfg_client_secret.clone(),
            tokens: tokens.clone(),
        },
    )?;

    // 3) Resolve the identifier (email) via the canonical primary-calendar endpoint.
    let http = GoogleClient::new();
    let identifier = match http.get_primary_calendar(&tokens.access_token).await {
        Ok(id) => id,
        Err(e) => {
            // Falls back to the list-based heuristic if the primary endpoint
            // fails (network blip, unusual permissions, etc.). Always
            // graceful — we'd rather show a slightly-wrong identifier than
            // refuse to add the account.
            tracing::warn!(error = %e, "calendars/primary failed; falling back to list");
            let cals = http.list_calendars(&tokens.access_token).await?;
            stint_core::calendar::google::resolve_account_identifier(&cals, &account_uuid)
        }
    };

    let account = CalendarAccount {
        id: account_uuid.clone(),
        provider: ProviderKind::Google,
        display_name: identifier.clone(),
        identifier,
        caldav_url: None,
        enabled: true,
        created_at: time::now_utc(),
    };
    cs.add_account(&account).await?;

    // 4) Initial refresh: on_add window, persist calendars + events.
    let provider = stint_core::calendar::google::build_provider_from_blob(&secrets, &account_uuid)?;
    let _ = refresh_account(&cs, &account_uuid, &*provider, Ranges::on_add()).await?;

    // 5) Notify the UI.
    let _ = app.emit(EVENT_CALENDAR_CHANGED, &account_uuid);
    Ok(account)
}

#[tauri::command]
pub async fn calendar_remove_account<R: Runtime>(
    state: State<'_, RwLock<AppState>>,
    app: tauri::AppHandle<R>,
    account_id: String,
) -> Result<(), AppError> {
    let store = store(&state).await;
    let cs = CalendarStore::new((*store).clone());
    cs.delete_account(&account_id).await?;
    let _ = calendar_blob_delete(&Secrets::default(), &account_id);
    let _ = app.emit(EVENT_CALENDAR_CHANGED, &account_id);
    Ok(())
}

#[tauri::command]
pub async fn calendar_list_calendars(
    state: State<'_, RwLock<AppState>>,
    account_id: String,
) -> Result<Vec<Calendar>, AppError> {
    let store = store(&state).await;
    let cs = CalendarStore::new((*store).clone());
    Ok(cs.list_calendars(&account_id).await?)
}

#[tauri::command]
pub async fn calendar_set_calendar_included<R: Runtime>(
    state: State<'_, RwLock<AppState>>,
    app: tauri::AppHandle<R>,
    calendar_id: String,
    included: bool,
) -> Result<(), AppError> {
    let store = store(&state).await;
    let cs = CalendarStore::new((*store).clone());
    cs.set_calendar_included(&calendar_id, included).await?;
    let _ = app.emit(EVENT_CALENDAR_CHANGED, &calendar_id);
    Ok(())
}

#[tauri::command]
pub async fn calendar_set_default_project<R: Runtime>(
    state: State<'_, RwLock<AppState>>,
    app: tauri::AppHandle<R>,
    calendar_id: String,
    project_id: Option<String>,
) -> Result<(), AppError> {
    let store = store(&state).await;
    let cs = CalendarStore::new((*store).clone());
    cs.set_default_project(&calendar_id, project_id.as_deref())
        .await?;
    let _ = app.emit(EVENT_CALENDAR_CHANGED, &calendar_id);
    Ok(())
}

#[tauri::command]
pub async fn calendar_refresh_account<R: Runtime>(
    state: State<'_, RwLock<AppState>>,
    app: tauri::AppHandle<R>,
    account_id: String,
) -> Result<usize, AppError> {
    let store = store(&state).await;
    let cs = CalendarStore::new((*store).clone());
    let secrets = Secrets::default();
    let provider = stint_core::calendar::google::build_provider_from_blob(&secrets, &account_id)?;
    let n = refresh_account(&cs, &account_id, &*provider, Ranges::on_focus()).await?;
    let _ = app.emit(EVENT_CALENDAR_CHANGED, &account_id);
    Ok(n)
}

#[tauri::command]
pub async fn calendar_list_events_in_range(
    state: State<'_, RwLock<AppState>>,
    account_id: String,
    from: String,
    to: String,
) -> Result<Vec<EventWithDecision>, AppError> {
    let store = store(&state).await;
    let cs = CalendarStore::new((*store).clone());
    let events = cs.list_events_in_range(&account_id, &from, &to).await?;
    let decisions = cs.list_decisions_in_range(&account_id, &from, &to).await?;
    Ok(events
        .into_iter()
        .map(|e| {
            let d = decisions
                .iter()
                .find(|(ev_id, start, _)| ev_id == &e.id && start == &e.start_at)
                .map(|(_, _, dec)| dec.clone());
            EventWithDecision {
                decision: d.as_ref().map(|d| d.as_wire().to_string()),
                linked_local_uuid: d
                    .as_ref()
                    .and_then(|d| d.linked_local_uuid().map(|s| s.to_string())),
                event: e,
            }
        })
        .collect())
}

#[tauri::command]
pub async fn calendar_log_event<R: Runtime>(
    state: State<'_, RwLock<AppState>>,
    app: tauri::AppHandle<R>,
    account_id: String,
    event_id: String,
    event_start: String,
) -> Result<String, AppError> {
    let store = store(&state).await;
    let cs = CalendarStore::new((*store).clone());
    let entries = Entries::new((*store).clone());

    let events = cs
        .list_events_in_range(&account_id, &event_start, &next_second(&event_start))
        .await?;
    let event = events
        .into_iter()
        .find(|e| e.id == event_id && e.start_at == event_start)
        .ok_or_else(|| AppError::msg("calendar event not found in store"))?;

    // Defense in depth: even if `calendar_events` has a stale row in offset
    // form (rows synced before the Google→Solidtime Z normalizer landed),
    // normalize here so the push to Solidtime always matches its `Y-m-d\TH:i:s\Z`
    // contract. All-day events pass through unchanged.
    let start_at = stint_core::time::to_solidtime_z(&event.start_at);
    let end_at = stint_core::time::to_solidtime_z(&event.end_at);

    // Look up the calendar's default project (if any). The picker on
    // Settings → Calendars controls this; it's a suggestion, not a lock.
    let default_project_id = cs
        .list_calendars(&account_id)
        .await?
        .into_iter()
        .find(|c| c.id == event.calendar_id)
        .and_then(|c| c.default_project_id);

    // When a default project is set, inherit its billable flag from the
    // cached Solidtime is_billable (refreshed on every sync tick). Without
    // a default project, fall back to non-billable.
    let billable = match default_project_id.as_deref() {
        Some(pid) => stint_core::store::reference::Reference::new((*store).clone())
            .list_projects()
            .await?
            .into_iter()
            .find(|p| p.id == pid)
            .map(|p| p.billable_default != 0)
            .unwrap_or(false),
        None => false,
    };

    let local_uuid = entries
        .create_completed(NewCompletedEntry {
            description: event.title,
            project_id: default_project_id,
            task_id: None,
            start_at,
            end_at,
            billable,
            source: "calendar".into(),
            source_event_id: Some(format!("{}:{}:{}", account_id, event.id, event.start_at)),
        })
        .await?;

    // create_completed writes the row with sync_state='pending_create' but
    // doesn't enqueue — mirroring TimerService::start, the caller is
    // responsible for inserting into sync_queue. Without this, the sync
    // drain never sees the entry and it never reaches Solidtime.
    let queue = Queue::new((*store).clone());
    queue
        .enqueue(
            QueueOp::CreateEntry,
            &serde_json::json!({ "local_uuid": local_uuid }).to_string(),
            Some(&local_uuid),
        )
        .await?;

    cs.record_decision(
        &account_id,
        &event_id,
        &event_start,
        &EventDecision::LoggedManual {
            linked_local_uuid: local_uuid.clone(),
        },
    )
    .await?;

    let _ = app.emit("entries:changed", 1);
    let _ = app.emit(EVENT_CALENDAR_CHANGED, &account_id);
    Ok(local_uuid)
}

#[tauri::command]
pub async fn calendar_ignore_event<R: Runtime>(
    state: State<'_, RwLock<AppState>>,
    app: tauri::AppHandle<R>,
    account_id: String,
    event_id: String,
    event_start: String,
) -> Result<(), AppError> {
    let store = store(&state).await;
    let cs = CalendarStore::new((*store).clone());
    cs.record_decision(
        &account_id,
        &event_id,
        &event_start,
        &EventDecision::Ignored,
    )
    .await?;
    let _ = app.emit(EVENT_CALENDAR_CHANGED, &account_id);
    Ok(())
}

/// Adds one second to an RFC 3339 timestamp so `list_events_in_range` can
/// be reused as a point-query. Falls back to the input if parsing fails.
fn next_second(ts: &str) -> String {
    match stint_core::time::parse(ts) {
        Ok(t) => stint_core::time::format(&(t + chrono::Duration::seconds(1))),
        Err(_) => ts.to_string(),
    }
}

fn open_url(url: &str) -> std::io::Result<()> {
    std::process::Command::new("/usr/bin/open")
        .arg(url)
        .status()
        .map(|_| ())
}
