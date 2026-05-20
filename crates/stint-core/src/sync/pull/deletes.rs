use crate::{
    solidtime::{dto::RemoteTimeEntry, SolidtimeClient},
    store::entries::Entries,
    Result,
};
use chrono::{DateTime, Utc};
use std::collections::HashSet;

const MAX_DELETE_PROBES_PER_PULL: usize = 50;

#[derive(Debug, Default, Clone, Copy)]
pub struct DeletesOutcome {
    pub deleted: usize,
}

/// For each local synced row in the window whose `solidtime_id` is NOT in the
/// list response, GET it by id. 404 → hard-delete locally. 200 → keep.
/// Capped at MAX_DELETE_PROBES_PER_PULL to bound worst-case cost.
pub async fn reconcile_deletes(
    conn: &mut sqlx::SqliteConnection,
    client: &SolidtimeClient,
    remote_entries: &[RemoteTimeEntry],
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<DeletesOutcome> {
    let from_str = from.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let to_str = to.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let local_rows = Entries::list_synced_in_window_with(&mut *conn, &from_str, &to_str).await?;

    let remote_ids: HashSet<&str> = remote_entries.iter().map(|e| e.id.as_str()).collect();

    let mut out = DeletesOutcome::default();
    let mut probes = 0;
    for row in local_rows {
        if probes >= MAX_DELETE_PROBES_PER_PULL {
            break;
        }
        let Some(solidtime_id) = row.solidtime_id.as_deref() else {
            continue;
        };
        if remote_ids.contains(solidtime_id) {
            continue;
        }
        probes += 1;
        if client.get_time_entry(solidtime_id).await?.is_none()
            && Entries::hard_delete_by_solidtime_id_with(&mut *conn, solidtime_id).await?
        {
            out.deleted += 1;
        }
    }
    Ok(out)
}
