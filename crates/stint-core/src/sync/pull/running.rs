use crate::{
    solidtime::{dto::RemoteTimeEntry, SolidtimeClient},
    store::{
        entries::{Entries, RemoteEntryUpsert},
        running::RunningTimer,
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
/// Runs on a borrowed sqlx connection (typically a transaction handle)
/// so the whole pull pipeline is atomic — any error rolls back the
/// running-timer mutations alongside the history changes.
pub async fn reconcile_running(
    conn: &mut sqlx::SqliteConnection,
    _client: &SolidtimeClient,
    remote_entries: &[RemoteTimeEntry],
) -> Result<RunningOutcome> {
    let remote_running = remote_entries.iter().find(|e| e.end.is_none());
    let local_running_row = RunningTimer::get_with(&mut *conn).await?;

    let local = match local_running_row {
        Some(r) => Entries::get_with(&mut *conn, &r.local_uuid).await?,
        None => None,
    };

    match (remote_running, local) {
        (None, _) => Ok(RunningOutcome::default()),
        (Some(remote), None) => {
            let local_uuid = Entries::create_from_remote_with(
                &mut *conn,
                RemoteEntryUpsert {
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
                },
            )
            .await?;
            RunningTimer::set_with(&mut *conn, &local_uuid).await?;
            Ok(RunningOutcome {
                adopted: Some(local_uuid),
                conflict: None,
            })
        }
        (Some(remote), Some(local_row)) => {
            if local_row.solidtime_id.as_deref() == Some(remote.id.as_str()) {
                Ok(RunningOutcome::default())
            } else {
                Ok(RunningOutcome {
                    adopted: None,
                    conflict: Some(ConflictInfo {
                        remote_id: remote.id.clone(),
                        remote_description: remote.description.clone(),
                        remote_start_at: remote.start.clone(),
                        local_local_uuid: local_row.local_uuid,
                        local_description: local_row.description,
                    }),
                })
            }
        }
    }
}
