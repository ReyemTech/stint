use crate::app_state::AppState;
use crate::commands::{store, AppError};
use serde::{Deserialize, Serialize};
use stint_core::store::entries::Entries;
use stint_core::store::running::RunningTimer;
use stint_core::timer::{StartArgs, TimerService};
use tauri::State;
use tokio::sync::RwLock;

#[derive(Serialize)]
pub struct RunningTimerView {
    pub local_uuid: String,
    pub description: String,
    pub start_at: String,
    pub project_id: Option<String>,
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
    }))
}

#[derive(Deserialize)]
pub struct StartTimerArgs {
    pub description: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
}

#[tauri::command]
pub async fn start_timer(
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
            source: "gui".into(),
        })
        .await?;
    Ok(id)
}

#[tauri::command]
pub async fn stop_timer(state: State<'_, RwLock<AppState>>) -> Result<String, AppError> {
    let store = store(&state).await;
    let timer = TimerService::new((*store).clone());
    Ok(timer.stop().await?)
}

#[tauri::command]
pub async fn delete_entry(
    state: State<'_, RwLock<AppState>>,
    local_uuid: String,
) -> Result<(), AppError> {
    let store = store(&state).await;
    let timer = TimerService::new((*store).clone());
    timer.delete(&local_uuid).await?;
    Ok(())
}

#[tauri::command]
pub async fn update_description(
    state: State<'_, RwLock<AppState>>,
    local_uuid: String,
    description: String,
) -> Result<(), AppError> {
    let store = store(&state).await;
    let timer = TimerService::new((*store).clone());
    timer.update_description(&local_uuid, &description).await?;
    Ok(())
}
