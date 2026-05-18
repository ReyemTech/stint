use crate::{
    solidtime::{dto::CreateEntryRequest, SolidtimeClient},
    store::{entries::Entries, queue::QueueRow, Store},
    Error, Result,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CreatePayload {
    local_uuid: String,
    description: String,
    project_id: Option<String>,
    task_id: Option<String>,
    start_at: String,
    #[serde(default)]
    billable: bool,
}

pub async fn push_one(
    store: &Store,
    client: &SolidtimeClient,
    row: &QueueRow,
) -> Result<()> {
    match row.op.as_str() {
        "create_entry" => push_create(store, client, row).await,
        other => Err(Error::Invariant(format!("unknown queue op: {other}"))),
    }
}

async fn push_create(
    store: &Store,
    client: &SolidtimeClient,
    row: &QueueRow,
) -> Result<()> {
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

    let queue = crate::store::queue::Queue::new(store.clone());
    queue.mark_succeeded(row.id).await?;
    Ok(())
}
