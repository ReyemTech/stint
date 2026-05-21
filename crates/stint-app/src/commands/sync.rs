use crate::app_state::AppState;
use crate::commands::{store, AppError};
use crate::sync_worker::EVENT_ENTRIES_CHANGED;
use serde::Serialize;
use stint_core::config::{secrets::Secrets, Settings};
use stint_core::solidtime::auth::build_token_provider;
use stint_core::solidtime::SolidtimeClient;
use stint_core::store::queue::{FailedQueueRow, Queue};
use stint_core::sync::{drain_once, refresh::refresh_reference_data};
use tauri::{AppHandle, Emitter, Runtime, State};
use tokio::sync::RwLock;

#[derive(Serialize)]
pub struct SyncErrorView {
    pub queue_id: i64,
    pub local_uuid: Option<String>,
    pub op: String,
    pub attempts: i64,
    pub last_error: Option<String>,
    pub next_try_at: String,
    /// True when next_try_at is >30 days out — i.e. mark_abandoned set it
    /// to ~1 year in the future. These need user attention; transient
    /// failures (5xx, timeouts) keep `abandoned=false` and self-recover.
    pub abandoned: bool,
    pub description: Option<String>,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
}

impl From<FailedQueueRow> for SyncErrorView {
    fn from(r: FailedQueueRow) -> Self {
        // Compute the abandoned flag at conversion time so the UI doesn't
        // have to know the 30-day convention.
        let cutoff = stint_core::time::format(&(chrono::Utc::now() + chrono::Duration::days(30)));
        let abandoned = r.next_try_at.as_str() > cutoff.as_str();
        SyncErrorView {
            queue_id: r.queue_id,
            local_uuid: r.local_uuid,
            op: r.op,
            attempts: r.attempts,
            last_error: r.last_error,
            next_try_at: r.next_try_at,
            abandoned,
            description: r.description,
            start_at: r.start_at,
            end_at: r.end_at,
        }
    }
}

#[tauri::command]
pub async fn list_sync_errors(
    state: State<'_, RwLock<AppState>>,
) -> Result<Vec<SyncErrorView>, AppError> {
    let store = store(&state).await;
    let rows = Queue::new((*store).clone())
        .list_failed_with_entry(3)
        .await?;
    Ok(rows.into_iter().map(SyncErrorView::from).collect())
}

#[tauri::command]
pub async fn sync_now<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RwLock<AppState>>,
) -> Result<usize, AppError> {
    let store = store(&state).await;
    let settings = Settings::new((*store).clone());
    let url = settings
        .get("solidtime.url")
        .await?
        .ok_or(stint_core::Error::MissingConfig("solidtime.url"))?;
    let secrets = Secrets::default();
    let org = settings
        .get("solidtime.org")
        .await?
        .ok_or(stint_core::Error::MissingConfig("solidtime.org"))?;
    let (provider, _oauth_client) = build_token_provider(&settings, &secrets, &url).await?;
    let client = SolidtimeClient::new(&url, provider).with_org(org);
    let n = drain_once(&store, &client).await?;
    // Reference refresh is best-effort: a transient projects/tasks failure
    // shouldn't mask a successful queue drain. The background sync loop
    // re-runs this every 15 ticks anyway, but users expect "Sync now" to
    // pick up project metadata changes (e.g. is_billable) on demand.
    if let Err(e) = refresh_reference_data(&store, &client).await {
        tracing::warn!(error = %e, "Sync now: reference refresh failed");
    }
    if n > 0 {
        let _ = app.emit(EVENT_ENTRIES_CHANGED, n);
    }
    Ok(n)
}
