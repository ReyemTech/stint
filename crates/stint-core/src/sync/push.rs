use crate::{
    solidtime::{dto::CreateEntryRequest, SolidtimeClient},
    store::{
        entries::Entries,
        queue::{Queue, QueueRow},
        Store,
    },
    Error, Result,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CreatePayload {
    local_uuid: String,
}

#[derive(Debug, Deserialize)]
struct DeletePayload {
    local_uuid: String,
    solidtime_id: String,
}

pub async fn push_one(store: &Store, client: &SolidtimeClient, row: &QueueRow) -> Result<()> {
    let result = match row.op.as_str() {
        "create_entry" => push_create(store, client, row).await,
        "update_entry" => push_update(store, client, row).await,
        "delete_entry" => push_delete(store, client, row).await,
        other => Err(Error::Invariant(format!("unknown queue op: {other}"))),
    };

    let queue = Queue::new(store.clone());
    match &result {
        Ok(()) => queue.mark_succeeded(row.id).await?,
        Err(e) => queue.mark_failed(row.id, &e.to_string()).await?,
    }
    result
}

async fn push_create(store: &Store, client: &SolidtimeClient, row: &QueueRow) -> Result<()> {
    let payload: CreatePayload = serde_json::from_str(&row.payload)?;
    let entries = Entries::new(store.clone());
    let current = entries
        .get(&payload.local_uuid)
        .await?
        .ok_or_else(|| Error::NotFound(format!("entry {}", payload.local_uuid)))?;

    let req = CreateEntryRequest {
        description: &current.description,
        project_id: current.project_id.as_deref(),
        task_id: current.task_id.as_deref(),
        start: &current.start_at,
        end: current.end_at.as_deref(),
        billable: current.billable != 0,
    };
    let remote = client.create_time_entry(&req).await?;
    entries.mark_synced(&payload.local_uuid, &remote.id).await?;
    Ok(())
}

async fn push_update(store: &Store, client: &SolidtimeClient, row: &QueueRow) -> Result<()> {
    let payload: CreatePayload = serde_json::from_str(&row.payload)?;
    let entries = Entries::new(store.clone());
    let current = entries
        .get(&payload.local_uuid)
        .await?
        .ok_or_else(|| Error::NotFound(format!("entry {}", payload.local_uuid)))?;
    let remote_id = current
        .solidtime_id
        .clone()
        .ok_or_else(|| Error::Invariant("update_entry without solidtime_id".into()))?;

    let req = CreateEntryRequest {
        description: &current.description,
        project_id: current.project_id.as_deref(),
        task_id: current.task_id.as_deref(),
        start: &current.start_at,
        end: current.end_at.as_deref(),
        billable: current.billable != 0,
    };
    client.update_time_entry(&remote_id, &req).await?;
    entries.mark_synced(&payload.local_uuid, &remote_id).await?;
    Ok(())
}

async fn push_delete(store: &Store, client: &SolidtimeClient, row: &QueueRow) -> Result<()> {
    let payload: DeletePayload = serde_json::from_str(&row.payload)?;
    client.delete_time_entry(&payload.solidtime_id).await?;

    // Hard-delete the local row now that the server agrees it's gone.
    let entries = Entries::new(store.clone());
    sqlx::query("DELETE FROM time_entries WHERE local_uuid = ?")
        .bind(&payload.local_uuid)
        .execute(store.pool())
        .await?;
    // Quiet warning: entries var would be unused otherwise.
    let _ = entries;
    Ok(())
}
