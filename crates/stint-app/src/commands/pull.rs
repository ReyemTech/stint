use crate::app_state::AppState;
use crate::commands::{store, AppError};
use crate::sync_worker::{EVENT_ENTRIES_CHANGED, EVENT_PULL_CONFLICT};
use serde::Serialize;
use stint_core::config::{secrets::Secrets, Settings};
use stint_core::solidtime::auth::build_token_provider;
use stint_core::solidtime::SolidtimeClient;
use stint_core::sync::pull::{pull, ConflictInfo, PullReport, Trigger};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::RwLock;

#[derive(Debug, Serialize)]
pub struct PullReportDto {
    pub adopted: Option<String>,
    pub conflict: Option<ConflictDto>,
    pub inserted: usize,
    pub updated: usize,
    pub deleted: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct ConflictDto {
    pub remote_id: String,
    pub remote_description: String,
    pub remote_start_at: String,
    pub local_local_uuid: String,
    pub local_description: String,
}

impl From<ConflictInfo> for ConflictDto {
    fn from(c: ConflictInfo) -> Self {
        Self {
            remote_id: c.remote_id,
            remote_description: c.remote_description,
            remote_start_at: c.remote_start_at,
            local_local_uuid: c.local_local_uuid,
            local_description: c.local_description,
        }
    }
}

impl From<PullReport> for PullReportDto {
    fn from(r: PullReport) -> Self {
        Self {
            adopted: r.adopted,
            conflict: r.conflict.map(ConflictDto::from),
            inserted: r.inserted,
            updated: r.updated,
            deleted: r.deleted,
        }
    }
}

#[tauri::command]
pub async fn pull_now(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
) -> Result<PullReportDto, AppError> {
    let store = store(&state).await;
    let settings = Settings::new((*store).clone());
    let Some(url) = settings.get("solidtime.url").await? else {
        return Err(AppError::msg("solidtime.url not set"));
    };
    let Some(org) = settings.get("solidtime.org").await? else {
        return Err(AppError::msg("solidtime.org not set"));
    };
    let secrets = Secrets::default();
    let (provider, _oauth_client) = build_token_provider(&settings, &secrets, &url).await?;
    let client = SolidtimeClient::new(&url, provider).with_org(org);

    let report = pull(&store, &client, Trigger::Manual).await?;
    if report.adopted.is_some() || report.inserted + report.updated + report.deleted > 0 {
        let _ = app.emit(EVENT_ENTRIES_CHANGED, 0u32);
    }
    if let Some(conflict) = &report.conflict {
        let _ = app.emit(EVENT_PULL_CONFLICT, ConflictDto::from(conflict.clone()));
    }
    Ok(PullReportDto::from(report))
}
