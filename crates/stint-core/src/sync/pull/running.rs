use crate::{
    solidtime::{dto::RemoteTimeEntry, SolidtimeClient},
    store::{
        entries::{Entries, RemoteEntryUpsert},
        running::RunningTimer,
        Store,
    },
    sync::pull::ConflictInfo,
    Result,
};

#[derive(Debug, Clone, Default)]
pub struct RunningOutcome {
    pub adopted: Option<String>,
    pub conflict: Option<ConflictInfo>,
}

/// Reconcile the local running timer against the (at most one) remote
/// running entry. See spec §6.
///
/// Task 6 implements only the (Some(remote), None) → ADOPT branch.
/// Other (Some, Some) cases land in Task 7.
pub async fn reconcile_running(
    store: &Store,
    _client: &SolidtimeClient,
    remote_entries: &[RemoteTimeEntry],
) -> Result<RunningOutcome> {
    let remote_running = remote_entries.iter().find(|e| e.end.is_none());
    let running = RunningTimer::new(store.clone());
    let local_running = running.get().await?;

    match (remote_running, local_running) {
        (None, _) => Ok(RunningOutcome::default()),
        (Some(remote), None) => {
            let entries = Entries::new(store.clone());
            let local_uuid = entries
                .create_from_remote(RemoteEntryUpsert {
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
            running.set(&local_uuid).await?;
            Ok(RunningOutcome {
                adopted: Some(local_uuid),
                conflict: None,
            })
        }
        (Some(_), Some(_)) => {
            // Handled in Task 7.
            Ok(RunningOutcome::default())
        }
    }
}
