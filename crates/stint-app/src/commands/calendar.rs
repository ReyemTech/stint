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
use stint_core::time;
use tauri::{Emitter, State};
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
pub async fn calendar_add_google(
    state: State<'_, RwLock<AppState>>,
    app: tauri::AppHandle,
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

    // 3) Resolve the identifier (email) via GoogleClient::list_calendars.
    let http = GoogleClient::new();
    let cals = http.list_calendars(&tokens.access_token).await?;
    let identifier = cals
        .iter()
        .find(|c| c.id == "primary")
        .map(|c| c.name.clone())
        .or_else(|| cals.first().map(|c| c.id.clone()))
        .unwrap_or_else(|| account_uuid.clone());

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
pub async fn calendar_remove_account(
    state: State<'_, RwLock<AppState>>,
    app: tauri::AppHandle,
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
pub async fn calendar_set_calendar_included(
    state: State<'_, RwLock<AppState>>,
    app: tauri::AppHandle,
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
pub async fn calendar_refresh_account(
    state: State<'_, RwLock<AppState>>,
    app: tauri::AppHandle,
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
pub async fn calendar_log_event(
    state: State<'_, RwLock<AppState>>,
    app: tauri::AppHandle,
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

    let local_uuid = entries
        .create_completed(NewCompletedEntry {
            description: event.title,
            project_id: None,
            task_id: None,
            start_at: event.start_at.clone(),
            end_at: event.end_at.clone(),
            billable: false,
            source: "calendar".into(),
            source_event_id: Some(format!("{}:{}:{}", account_id, event.id, event.start_at)),
        })
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
pub async fn calendar_ignore_event(
    state: State<'_, RwLock<AppState>>,
    app: tauri::AppHandle,
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
