use crate::{
    solidtime::dto::RemoteTimeEntry,
    store::entries::{Entries, RemoteEntryUpsert, TimeEntryRow},
    time, Result,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct HistoryOutcome {
    pub inserted: usize,
    pub updated: usize,
}

/// Reconcile completed entries (entries whose `end` is set). Inserts new
/// remote-only rows; updates existing `synced` rows when any field differs
/// from the remote; skips local rows with pending mutations. See spec §8.
///
/// Note on `updated_at`: Solidtime's list endpoint does not include
/// `updated_at` on time-entry rows, so we can't compare timestamps
/// reliably. Instead we field-compare local against remote and only
/// update when something actually changed — that's idempotent and
/// catches the "stopped externally" case (local end_at None, remote
/// end_at set).
pub async fn reconcile_history(
    conn: &mut sqlx::SqliteConnection,
    remote_entries: &[RemoteTimeEntry],
) -> Result<HistoryOutcome> {
    let mut out = HistoryOutcome::default();

    for remote in remote_entries.iter().filter(|e| e.end.is_some()) {
        let existing = Entries::get_by_solidtime_id_with(&mut *conn, &remote.id).await?;
        let upsert = RemoteEntryUpsert {
            solidtime_id: remote.id.clone(),
            description: remote.description.clone(),
            project_id: remote.project_id.clone(),
            task_id: remote.task_id.clone(),
            start_at: remote.start.clone(),
            end_at: remote.end.clone(),
            billable: remote.billable,
            updated_at: remote.updated_at.clone().unwrap_or_else(time::now_utc),
        };

        match existing {
            None => {
                Entries::create_from_remote_with(&mut *conn, upsert).await?;
                out.inserted += 1;
            }
            Some(local) => {
                if local.sync_state != "synced" {
                    continue;
                }
                if !fields_differ(&local, remote) {
                    continue;
                }
                if Entries::update_from_remote_with(&mut *conn, &remote.id, upsert).await? {
                    out.updated += 1;
                }
            }
        }
    }
    Ok(out)
}

/// Compare the user-observable fields between a local row and the remote
/// payload. Used in place of an `updated_at` comparison because Solidtime
/// omits `updated_at` from the list endpoint.
fn fields_differ(local: &TimeEntryRow, remote: &RemoteTimeEntry) -> bool {
    local.description != remote.description
        || local.project_id != remote.project_id
        || local.task_id != remote.task_id
        || local.start_at != remote.start
        || local.end_at != remote.end
        || (local.billable != 0) != remote.billable
}
