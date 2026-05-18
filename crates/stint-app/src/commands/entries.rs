use crate::app_state::AppState;
use crate::commands::{store, AppError};
use chrono::{Local, TimeZone, Utc};
use serde::Serialize;
use stint_core::store::entries::{Entries, TimeEntryRow};
use tauri::State;
use tokio::sync::RwLock;

#[derive(Serialize)]
pub struct EntryView {
    pub local_uuid: String,
    pub solidtime_id: Option<String>,
    pub description: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub start_at: String,
    pub end_at: Option<String>,
    pub sync_state: String,
    pub source: String,
}

impl From<TimeEntryRow> for EntryView {
    fn from(r: TimeEntryRow) -> Self {
        Self {
            local_uuid: r.local_uuid,
            solidtime_id: r.solidtime_id,
            description: r.description,
            project_id: r.project_id,
            task_id: r.task_id,
            start_at: r.start_at,
            end_at: r.end_at,
            sync_state: r.sync_state,
            source: r.source,
        }
    }
}

#[tauri::command]
pub async fn list_today(
    state: State<'_, RwLock<AppState>>,
) -> Result<Vec<EntryView>, AppError> {
    let store = store(&state).await;
    let today = Local::now().date_naive();
    let start_local = Local
        .from_local_datetime(&today.and_hms_opt(0, 0, 0).unwrap())
        .unwrap();
    let end_local = start_local + chrono::Duration::days(1);
    let from = start_local.with_timezone(&Utc).to_rfc3339();
    let to = end_local.with_timezone(&Utc).to_rfc3339();

    let entries = Entries::new((*store).clone());
    let rows = entries.list_between(&from, &to).await?;
    Ok(rows.into_iter().map(EntryView::from).collect())
}

#[tauri::command]
pub async fn list_between(
    state: State<'_, RwLock<AppState>>,
    from: String,
    to: String,
) -> Result<Vec<EntryView>, AppError> {
    let store = store(&state).await;
    let entries = Entries::new((*store).clone());
    let rows = entries.list_between(&from, &to).await?;
    Ok(rows.into_iter().map(EntryView::from).collect())
}
