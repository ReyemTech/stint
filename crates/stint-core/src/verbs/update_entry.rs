//! Apply a partial patch to an existing entry.
//!
//! Uses the existing field-level setters on [`Entries`] rather than a single
//! dynamic-SQL `apply_patch`. Each setter routes through `update_one` which
//! transitions `sync_state` (synced → dirty, pending_create stays put), so
//! reusing them gives us correct sync semantics for free.
//!
//! `EntryPatch` uses `Option<Option<T>>` for nullable fields to preserve the
//! 3-way distinction between "no change", "clear", and "set" across the wire.
//! See `verbs::types::EntryPatch` for the encoding contract.

use crate::store::entries::Entries;
use crate::store::queue::{Queue, QueueOp};
use crate::store::Store;
use crate::verbs::types::{EntryPatch, EntryView};
use crate::{Error, Result};

pub async fn update_entry(store: &Store, local_uuid: &str, patch: EntryPatch) -> Result<EntryView> {
    let entries = Entries::new(store.clone());

    // Fail fast if the row doesn't exist — every setter would otherwise no-op
    // silently against a missing local_uuid and we'd return a confusing
    // "expected row missing" later on.
    let existing = entries
        .get(local_uuid)
        .await?
        .ok_or_else(|| Error::NotFound(format!("entry {local_uuid}")))?;

    if let Some(desc) = patch.description.as_deref() {
        entries.update_description(local_uuid, desc).await?;
    }

    match patch.project_id {
        None => {}
        Some(None) => entries.set_project(local_uuid, None).await?,
        Some(Some(ref v)) => entries.set_project(local_uuid, Some(v)).await?,
    }

    match patch.task_id {
        None => {}
        Some(None) => entries.set_task(local_uuid, None).await?,
        Some(Some(ref v)) => entries.set_task(local_uuid, Some(v)).await?,
    }

    if let Some(b) = patch.billable {
        entries.set_billable(local_uuid, b).await?;
    }

    // start_at / end_at are coupled: `update_times` validates the pair (end
    // after start, ≤24h). When only one side changes we substitute the
    // existing value to keep the invariant check honest. When end_at is being
    // cleared explicitly (Some(None)), a start_at change is applied via
    // `update_times` against the still-present old end first — but if the
    // old row has no end yet, that path is unavailable; we fall through to
    // `clear_end` and only update start_at via a fresh load.
    match (&patch.start_at, &patch.end_at) {
        (None, None) => {}
        (Some(start), Some(Some(end))) => {
            entries.update_times(local_uuid, start, end).await?;
        }
        (Some(start), None) => {
            let end = existing.end_at.as_deref().ok_or_else(|| {
                Error::Invariant(
                    "cannot set start_at on a running entry without also setting end_at".into(),
                )
            })?;
            entries.update_times(local_uuid, start, end).await?;
        }
        (None, Some(Some(end))) => {
            entries.set_end(local_uuid, end).await?;
        }
        (None, Some(None)) => {
            entries.clear_end(local_uuid).await?;
        }
        (Some(_), Some(None)) => {
            return Err(Error::Invariant(
                "cannot set start_at while clearing end_at; clear first then update".into(),
            ));
        }
    }

    let row = entries
        .get(local_uuid)
        .await?
        .ok_or_else(|| Error::NotFound(format!("entry {local_uuid}")))?;

    // If a previously-synced entry was transitioned to "dirty" by one of the
    // setters above, queue an update op so the sync worker reconciles with
    // Solidtime. Mirrors the maybe_enqueue_update path in TimerService so the
    // verb can replace the service for transports (CLI / Tauri / MCP).
    if row.sync_state == "dirty" {
        let queue = Queue::new(store.clone());
        let payload = serde_json::to_string(&row)?;
        queue
            .enqueue(QueueOp::UpdateEntry, &payload, Some(local_uuid))
            .await?;
    }

    Ok(row.into())
}
