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

#[derive(Serialize)]
pub struct OverlapCandidate {
    pub id: String,
    pub description: String,
    pub start: String,
    /// `None` means the remote is still running.
    pub end: Option<String>,
}

/// For a local entry that's failing to sync (most often overlapping_time_entry),
/// fetch Solidtime entries whose time range actually intersects the local
/// row's [start, end]. Used by the in-app sync-error banner to tell the
/// user "this conflicts with X" instead of just "this conflicts".
#[tauri::command]
pub async fn get_sync_error_overlaps(
    state: State<'_, RwLock<AppState>>,
    local_uuid: String,
) -> Result<Vec<OverlapCandidate>, AppError> {
    let store = store(&state).await;
    let entries = stint_core::store::entries::Entries::new((*store).clone());
    let Some(entry) = entries.get(&local_uuid).await? else {
        return Ok(Vec::new());
    };

    let settings = Settings::new((*store).clone());
    let url = match settings.get("solidtime.url").await? {
        Some(u) => u,
        None => return Ok(Vec::new()),
    };
    let Some(org) = settings.get("solidtime.org").await? else {
        return Ok(Vec::new());
    };
    let Some(member_id) = settings.get("solidtime.member_id").await? else {
        return Ok(Vec::new());
    };
    let secrets = Secrets::default();
    let (provider, _oauth_client) = build_token_provider(&settings, &secrets, &url).await?;
    let client = SolidtimeClient::new(&url, provider).with_org(org);

    let our_start = stint_core::time::parse(&entry.start_at)?;
    let our_end = match entry.end_at.as_deref() {
        Some(e) => stint_core::time::parse(e)?,
        None => stint_core::time::now(),
    };

    // Query a 24-hour-before window so completed entries that started
    // earlier the same day land in the result set.
    let from = stint_core::time::format(&(our_start - chrono::Duration::hours(24)));
    let to = stint_core::time::format(&(our_end + chrono::Duration::seconds(1)));
    let mut candidates = client
        .list_time_entries(&member_id, &from, &to)
        .await
        .unwrap_or_default();
    // Active=true entries can have a start before our 24h window but still
    // be running — add them in too.
    if let Ok(active) = client.list_active_time_entries(&member_id).await {
        for a in active {
            if !candidates.iter().any(|c| c.id == a.id) {
                candidates.push(a);
            }
        }
    }

    let mut out = Vec::new();
    for r in candidates {
        let r_start = match stint_core::time::parse(&r.start) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let r_end = match r.end.as_deref() {
            Some(e) => stint_core::time::parse(e).unwrap_or(our_end),
            None => stint_core::time::now(),
        };
        // Range-intersection: their [start, end] crosses our [start, end].
        if r_start < our_end && r_end > our_start {
            out.push(OverlapCandidate {
                id: r.id,
                description: r.description,
                start: r.start,
                end: r.end,
            });
        }
    }
    Ok(out)
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
