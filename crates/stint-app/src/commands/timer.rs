use crate::app_state::AppState;
use crate::commands::{store, AppError};
use crate::sync_worker::{self, EVENT_ENTRIES_CHANGED};
use serde::{Deserialize, Serialize};
use stint_core::store::entries::Entries;
use stint_core::store::running::RunningTimer;
use stint_core::timer::{StartArgs, TimerService};
use tauri::{AppHandle, Emitter, Runtime, State};
use tokio::sync::RwLock;

fn announce_change<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.emit(EVENT_ENTRIES_CHANGED, ());
}

#[derive(Serialize)]
pub struct RunningTimerView {
    pub local_uuid: String,
    pub description: String,
    pub start_at: String,
    pub project_id: Option<String>,
    pub billable: bool,
}

#[tauri::command]
pub async fn get_running_timer(
    state: State<'_, RwLock<AppState>>,
) -> Result<Option<RunningTimerView>, AppError> {
    let store = store(&state).await;
    let running = RunningTimer::new((*store).clone());
    let Some(r) = running.get().await? else {
        return Ok(None);
    };
    let entries = Entries::new((*store).clone());
    let entry = entries.get(&r.local_uuid).await?;
    Ok(entry.map(|e| RunningTimerView {
        local_uuid: e.local_uuid,
        description: e.description,
        start_at: e.start_at,
        project_id: e.project_id,
        billable: e.billable != 0,
    }))
}

#[derive(Deserialize)]
pub struct StartTimerArgs {
    pub description: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    #[serde(default)]
    pub billable: bool,
}

#[tauri::command]
pub async fn start_timer<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RwLock<AppState>>,
    args: StartTimerArgs,
) -> Result<String, AppError> {
    let store = store(&state).await;
    let timer = TimerService::new((*store).clone());
    let id = timer
        .start(StartArgs {
            description: args.description,
            project_id: args.project_id,
            task_id: args.task_id,
            billable: args.billable,
            source: "gui".into(),
            start_at: None,
        })
        .await?;
    announce_change(&app);
    sync_worker::nudge(app.clone(), store);
    Ok(id)
}

#[tauri::command]
pub async fn stop_timer<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RwLock<AppState>>,
) -> Result<String, AppError> {
    let store = store(&state).await;
    let timer = TimerService::new((*store).clone());
    let id = timer.stop().await?;
    announce_change(&app);
    sync_worker::nudge(app.clone(), store);
    Ok(id)
}

#[tauri::command]
pub async fn delete_entry<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RwLock<AppState>>,
    local_uuid: String,
) -> Result<(), AppError> {
    let store = store(&state).await;
    let timer = TimerService::new((*store).clone());
    timer.delete(&local_uuid).await?;
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
    let timer = TimerService::new((*store).clone());
    timer.update_description(&local_uuid, &description).await?;
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
    let timer = TimerService::new((*store).clone());
    timer
        .set_project(&local_uuid, project_id.as_deref())
        .await?;
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
    let timer = TimerService::new((*store).clone());
    timer.set_billable(&local_uuid, billable).await?;
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
    let timer = TimerService::new((*store).clone());
    timer.update_times(&local_uuid, &start_at, &end_at).await?;
    announce_change(&app);
    sync_worker::nudge(app.clone(), store);
    Ok(())
}
