use crate::store::entries::Entries;
use crate::store::Store;
use crate::timer::TimerService;
use crate::verbs::types::EntryView;
use crate::Result;

/// Stop the currently running timer. Errors if no timer is running.
pub async fn stop(store: &Store) -> Result<EntryView> {
    let timer = TimerService::new(store.clone());
    let id = timer.stop().await?;
    let entries = Entries::new(store.clone());
    let row = entries
        .get(&id)
        .await?
        .expect("just-stopped entry must exist");

    let view: EntryView = row.into();
    if let Ok(payload) = serde_json::to_string(&view) {
        crate::ffi::notify_indexer(crate::ffi::IndexerKind::EntryStopped, &payload);
    }
    Ok(view)
}
