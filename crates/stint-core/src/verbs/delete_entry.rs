//! Delete an entry by `local_uuid`.
//!
//! Routes through [`TimerService::delete`] — the same primitive the CLI
//! (`stint delete`) and Tauri (`commands::timer::delete`) already use — so
//! the sync-queue side-effects stay correct:
//!
//! * If the row never reached Solidtime (`pending_create`), the pending
//!   `CreateEntry` queue op is removed and the row is hard-deleted.
//! * If the row was synced, a `DeleteEntry` op is enqueued and the row is
//!   marked `pending_delete` so the worker can flush it remotely.
//!
//! Going straight to `Entries::delete` would skip queue management and
//! leave synced rows stuck locally as `pending_delete` with no op to drain
//! them.
//!
//! Idempotent: if the row is already gone, returns `Ok(())` instead of
//! `Error::NotFound`. Callers that need to distinguish should check with
//! `Entries::get` first.

use crate::store::entries::Entries;
use crate::store::Store;
use crate::timer::TimerService;
use crate::{Error, Result};

pub async fn delete_entry(store: &Store, local_uuid: &str) -> Result<()> {
    // Idempotency probe — `TimerService::delete` errors with NotFound on a
    // missing row, but the verb contract is "ensure it's gone".
    let entries = Entries::new(store.clone());
    if entries.get(local_uuid).await?.is_none() {
        return Ok(());
    }

    let timer = TimerService::new(store.clone());
    let result = match timer.delete(local_uuid).await {
        Ok(()) => Ok(()),
        // Race: row vanished between the probe and the delete. Still a
        // success from the verb's point of view.
        Err(Error::NotFound(_)) => Ok(()),
        Err(e) => Err(e),
    };

    if result.is_ok() {
        let payload = serde_json::json!({ "local_uuid": local_uuid }).to_string();
        crate::ffi::notify_indexer(crate::ffi::IndexerKind::EntryDeleted, &payload);
    }
    result
}
