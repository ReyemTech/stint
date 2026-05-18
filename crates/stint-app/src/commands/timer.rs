use crate::app_state::AppState;
use crate::commands::{store, AppError};
use serde::Serialize;
use stint_core::store::entries::Entries;
use stint_core::store::running::RunningTimer;
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
