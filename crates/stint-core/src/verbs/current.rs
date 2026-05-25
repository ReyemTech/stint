use crate::store::entries::Entries;
use crate::store::running::RunningTimer;
use crate::store::Store;
use crate::verbs::types::EntryView;
use crate::Result;

/// Return the currently running entry, or None when idle.
pub async fn current(store: &Store) -> Result<Option<EntryView>> {
    let running = RunningTimer::new(store.clone());
    let Some(r) = running.get().await? else {
        return Ok(None);
    };
    let entries = Entries::new(store.clone());
    let Some(row) = entries.get(&r.local_uuid).await? else {
        return Ok(None);
    };
    Ok(Some(row.into()))
}
