//! Tauri timer commands. Every command here is a thin wrapper that
//! delegates business logic to `stint_core::verbs::*` (the single source of
//! truth shared with the CLI, MCP, and HTTP transports) and returns the
//! verbs' canonical `EntryView` shape to the UI.
//!
//! The transport-only concerns this layer keeps are:
//!   * emitting the `entries:changed` event so other UI windows refresh
//!   * nudging the background sync worker after a write
//!   * adapting the verb error type into `AppError` (via `?`)
//!
//! For the individual setters (`update_description`, `set_entry_project`,
//! `set_entry_billable`, `update_entry_times`) we still expose granular
//! Tauri commands so existing UI call sites keep working, but each one
//! constructs a small `EntryPatch` and delegates to `verbs::update_entry`
//! — this means sync_state transitions are managed in exactly one place.

use crate::app_state::AppState;
use crate::commands::{store, AppError};
use crate::sync_worker::{self, EVENT_ENTRIES_CHANGED};
use serde::Deserialize;
use stint_core::store::entries::Entries;
use stint_core::verbs::{self, EntryPatch, EntryView, StartParams};
use tauri::{AppHandle, Emitter, Runtime, State};
use tokio::sync::RwLock;

fn announce_change<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.emit(EVENT_ENTRIES_CHANGED, ());
}

#[tauri::command]
pub async fn get_running_timer(
    state: State<'_, RwLock<AppState>>,
) -> Result<Option<EntryView>, AppError> {
    let store = store(&state).await;
    Ok(verbs::current(&store).await?)
}

#[derive(Deserialize)]
pub struct StartTimerArgs {
    pub description: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    #[serde(default)]
    pub billable: bool,
    /// Optional ISO 8601 UTC timestamp. None → start "now". Rejected if in
    /// the future (validated downstream by the verb).
    #[serde(default)]
    pub start_at: Option<String>,
}

impl StartTimerArgs {
    fn into_params(self) -> StartParams {
        StartParams {
            description: self.description,
            project_id: self.project_id,
            task_id: self.task_id,
            billable: self.billable,
            source: "gui".into(),
            start_at: self.start_at,
        }
    }
}

#[tauri::command]
pub async fn start_timer<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RwLock<AppState>>,
    args: StartTimerArgs,
) -> Result<EntryView, AppError> {
    let store = store(&state).await;
    let view = verbs::start(&store, args.into_params()).await?;
    announce_change(&app);
    sync_worker::nudge(app.clone(), store);
    Ok(view)
}

/// Start a fresh timer using the description / project / task / billable
/// from an existing entry. If a timer is already running, stop it first so
/// the user can "click to repeat" in one step.
#[tauri::command]
pub async fn restart_entry<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RwLock<AppState>>,
    local_uuid: String,
) -> Result<EntryView, AppError> {
    let store = store(&state).await;
    let entries = Entries::new((*store).clone());
    let template = entries
        .get(&local_uuid)
        .await?
        .ok_or_else(|| stint_core::Error::NotFound(format!("entry {local_uuid}")))?;

    // Best-effort stop of any in-flight timer so the start below doesn't
    // collide. Ignore "no timer running" — that's the expected idle case.
    let _ = verbs::stop(&store).await;

    let view = verbs::start(
        &store,
        StartParams {
            description: template.description,
            project_id: template.project_id,
            task_id: template.task_id,
            billable: template.billable != 0,
            source: "gui".into(),
            start_at: None,
        },
    )
    .await?;
    announce_change(&app);
    sync_worker::nudge(app.clone(), store);
    Ok(view)
}

#[tauri::command]
pub async fn stop_timer<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RwLock<AppState>>,
) -> Result<EntryView, AppError> {
    let store = store(&state).await;
    let view = verbs::stop(&store).await?;
    announce_change(&app);
    sync_worker::nudge(app.clone(), store);
    Ok(view)
}

#[tauri::command]
pub async fn delete_entry<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RwLock<AppState>>,
    local_uuid: String,
) -> Result<(), AppError> {
    let store = store(&state).await;
    verbs::delete_entry(&store, &local_uuid).await?;
    announce_change(&app);
    sync_worker::nudge(app.clone(), store);
    Ok(())
}

#[tauri::command]
pub async fn update_description<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RwLock<AppState>>,
    local_uuid: String,
    description: String,
) -> Result<(), AppError> {
    let store = store(&state).await;
    let patch = EntryPatch {
        description: Some(description),
        ..Default::default()
    };
    verbs::update_entry(&store, &local_uuid, patch).await?;
    announce_change(&app);
    sync_worker::nudge(app.clone(), store);
    Ok(())
}

#[tauri::command]
pub async fn set_entry_project<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RwLock<AppState>>,
    local_uuid: String,
    project_id: Option<String>,
) -> Result<(), AppError> {
    let store = store(&state).await;
    // Preserve the existing "null = clear, value = set" semantics over the
    // wire by lifting the Option into the 3-way Option<Option<T>> patch.
    // The Tauri arg has no distinct "absent" state, so we always pass Some(...).
    let patch = EntryPatch {
        project_id: Some(project_id),
        ..Default::default()
    };
    verbs::update_entry(&store, &local_uuid, patch).await?;
    announce_change(&app);
    sync_worker::nudge(app.clone(), store);
    Ok(())
}

#[tauri::command]
pub async fn set_entry_billable<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RwLock<AppState>>,
    local_uuid: String,
    billable: bool,
) -> Result<(), AppError> {
    let store = store(&state).await;
    let patch = EntryPatch {
        billable: Some(billable),
        ..Default::default()
    };
    verbs::update_entry(&store, &local_uuid, patch).await?;
    announce_change(&app);
    sync_worker::nudge(app.clone(), store);
    Ok(())
}

#[tauri::command]
pub async fn update_entry_times<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RwLock<AppState>>,
    local_uuid: String,
    start_at: String,
    end_at: String,
) -> Result<(), AppError> {
    let store = store(&state).await;
    let patch = EntryPatch {
        start_at: Some(start_at),
        end_at: Some(Some(end_at)),
        ..Default::default()
    };
    verbs::update_entry(&store, &local_uuid, patch).await?;
    announce_change(&app);
    sync_worker::nudge(app.clone(), store);
    Ok(())
}
