use crate::{
    solidtime::dto::RemoteTimeEntry,
    store::{
        entries::{Entries, RemoteEntryUpsert},
        Store,
    },
    Result,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct HistoryOutcome {
    pub inserted: usize,
    pub updated: usize,
}

/// Reconcile completed entries (entries whose `end` is set). Inserts new
/// remote-only rows; updates existing `synced` rows when remote is newer;
/// skips local rows with pending mutations. See spec §8.
pub async fn reconcile_history(
    store: &Store,
    remote_entries: &[RemoteTimeEntry],
) -> Result<HistoryOutcome> {
    let entries = Entries::new(store.clone());
    let mut out = HistoryOutcome::default();

    for remote in remote_entries.iter().filter(|e| e.end.is_some()) {
        let existing = entries.get_by_solidtime_id(&remote.id).await?;
        let upsert = RemoteEntryUpsert {
            solidtime_id: remote.id.clone(),
            description: remote.description.clone(),
            project_id: remote.project_id.clone(),
            task_id: remote.task_id.clone(),
            start_at: remote.start.clone(),
            end_at: remote.end.clone(),
            billable: remote.billable,
            updated_at: remote
                .updated_at
                .clone()
                .unwrap_or_else(|| remote.start.clone()),
        };

        match existing {
            None => {
                entries.create_from_remote(upsert).await?;
                out.inserted += 1;
            }
            Some(local) => {
                if local.sync_state != "synced" {
                    continue;
                }
                if !is_remote_newer(&local.updated_at, &upsert.updated_at) {
                    continue;
                }
                if entries.update_from_remote(&remote.id, upsert).await? {
                    out.updated += 1;
                }
            }
        }
    }
    Ok(out)
}

fn is_remote_newer(local_updated_at: &str, remote_updated_at: &str) -> bool {
    remote_updated_at > local_updated_at
}
