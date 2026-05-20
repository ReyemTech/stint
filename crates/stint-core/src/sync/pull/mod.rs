//! Down-sync from Solidtime: running-timer adoption, history & delete
//! reconciliation. Each trigger calls `pull(...)` which runs the
//! reconciliation sub-functions and returns a summary. Task 6 wires up
//! running-timer adoption; history and deletes land in subsequent tasks.

pub mod history;
pub mod running;
pub mod window;

pub use window::{Trigger, Window};

use crate::{
    config::Settings,
    solidtime::SolidtimeClient,
    store::Store,
    Error, Result,
};
use chrono::Utc;

#[derive(Debug, Default, Clone)]
pub struct PullReport {
    pub adopted: Option<String>,
    pub conflict: Option<ConflictInfo>,
    pub inserted: usize,
    pub updated: usize,
    pub deleted: usize,
}

#[derive(Debug, Clone)]
pub struct ConflictInfo {
    pub remote_id: String,
    pub remote_description: String,
    pub remote_start_at: String,
    pub local_local_uuid: String,
    pub local_description: String,
}

pub async fn pull(
    store: &Store,
    client: &SolidtimeClient,
    trigger: Trigger,
) -> Result<PullReport> {
    let settings = Settings::new(store.clone());
    let member_id = settings
        .get("solidtime.member_id")
        .await?
        .ok_or(Error::MissingConfig("solidtime.member_id"))?;

    let window = Window::for_trigger(trigger, Utc::now());
    let from = window.from.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let to = window.to.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let remote_entries = client.list_time_entries(&member_id, &from, &to).await?;

    let running_outcome = running::reconcile_running(store, client, &remote_entries).await?;
    let history_outcome = history::reconcile_history(store, &remote_entries).await?;

    Ok(PullReport {
        adopted: running_outcome.adopted,
        conflict: running_outcome.conflict,
        inserted: history_outcome.inserted,
        updated: history_outcome.updated,
        deleted: 0,
    })
}

#[derive(Debug, Clone, Copy)]
pub enum ConflictAction {
    StopRemote,
    Switch,
    Dismiss,
}

/// Resolve a running-timer conflict surfaced by an earlier `pull` call.
///
/// - `Dismiss` — no-op (caller is acknowledging the conflict; banner closes).
/// - `StopRemote` — fetches the remote running entry, mirrors it locally as
///   `synced`, sets its end_at to now, and enqueues an UpdateEntry so the
///   sync queue pushes the end to Solidtime. Local timer is untouched.
/// - `Switch` — stops the local timer (normal flow), then runs another pull
///   so the now-empty local-running slot adopts the remote.
pub async fn resolve_conflict(
    store: &Store,
    client: &SolidtimeClient,
    action: ConflictAction,
    remote_id: &str,
) -> Result<()> {
    match action {
        ConflictAction::Dismiss => Ok(()),
        ConflictAction::StopRemote => {
            let remote = client
                .get_time_entry(remote_id)
                .await?
                .ok_or_else(|| Error::NotFound(format!("remote entry {remote_id}")))?;

            let entries = crate::store::entries::Entries::new(store.clone());
            let local_uuid = entries
                .create_from_remote(crate::store::entries::RemoteEntryUpsert {
                    solidtime_id: remote.id.clone(),
                    description: remote.description.clone(),
                    project_id: remote.project_id.clone(),
                    task_id: remote.task_id.clone(),
                    start_at: remote.start.clone(),
                    end_at: None,
                    billable: remote.billable,
                    updated_at: remote
                        .updated_at
                        .clone()
                        .unwrap_or_else(|| remote.start.clone()),
                })
                .await?;
            let now = crate::time::now_utc();
            entries.set_end(&local_uuid, &now).await?;

            let queue = crate::store::queue::Queue::new(store.clone());
            queue
                .enqueue(
                    crate::store::queue::QueueOp::UpdateEntry,
                    &serde_json::json!({ "local_uuid": local_uuid }).to_string(),
                    Some(&local_uuid),
                )
                .await?;
            Ok(())
        }
        ConflictAction::Switch => {
            crate::timer::TimerService::new(store.clone()).stop().await?;
            pull(store, client, Trigger::Manual).await.map(|_| ())
        }
    }
}
