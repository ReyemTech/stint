use crate::{
    config::Settings,
    solidtime::{dto::CreateEntryRequest, SolidtimeClient},
    store::{
        entries::Entries,
        queue::{Queue, QueueRow},
        running::RunningTimer,
        Store,
    },
    Error, Result,
};
use serde::Deserialize;
use tracing::{debug, info};

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

async fn member_id(store: &Store) -> Result<String> {
    Settings::new(store.clone())
        .get("solidtime.member_id")
        .await?
        .ok_or(Error::MissingConfig("solidtime.member_id"))
}

async fn push_create(store: &Store, client: &SolidtimeClient, row: &QueueRow) -> Result<()> {
    let payload: CreatePayload = serde_json::from_str(&row.payload)?;
    let entries = Entries::new(store.clone());
    let current = entries
        .get(&payload.local_uuid)
        .await?
        .ok_or_else(|| Error::NotFound(format!("entry {}", payload.local_uuid)))?;

    let member = member_id(store).await?;
    let req = CreateEntryRequest {
        member_id: &member,
        description: &current.description,
        project_id: current.project_id.as_deref(),
        task_id: current.task_id.as_deref(),
        start: &current.start_at,
        end: current.end_at.as_deref(),
        billable: current.billable != 0,
    };
    debug!(?req, "create_time_entry request");
    let remote = client.create_time_entry(&req).await?;
    info!(local = %payload.local_uuid, remote = %remote.id, "create_entry synced");
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

    let member = member_id(store).await?;
    let req = CreateEntryRequest {
        member_id: &member,
        description: &current.description,
        project_id: current.project_id.as_deref(),
        task_id: current.task_id.as_deref(),
        start: &current.start_at,
        end: current.end_at.as_deref(),
        billable: current.billable != 0,
    };
    debug!(?req, remote = %remote_id, "update_time_entry request");
    match client.update_time_entry(&remote_id, &req).await {
        Ok(_) => {
            info!(local = %payload.local_uuid, remote = %remote_id, "update_entry synced");
            entries.mark_synced(&payload.local_uuid, &remote_id).await?;
            Ok(())
        }
        Err(Error::Solidtime { status: 404, .. }) => {
            // Remote was deleted out from under us (e.g. user deleted the
            // entry directly in Solidtime). Mirror the deletion locally
            // instead of retrying forever.
            info!(
                local = %payload.local_uuid, remote = %remote_id,
                "update_entry: remote gone (404), deleting local row",
            );
            handle_remote_gone(store, &payload.local_uuid).await
        }
        Err(e) => Err(e),
    }
}

/// Mirror a server-side deletion locally: drop the time_entries row and
/// clear running_timer if it pointed at this row. Used when push observes
/// a 404 from PUT/DELETE — the queue op succeeds, no retry.
async fn handle_remote_gone(store: &Store, local_uuid: &str) -> Result<()> {
    let running = RunningTimer::new(store.clone());
    if let Some(r) = running.get().await? {
        if r.local_uuid == local_uuid {
            running.clear().await?;
        }
    }
    sqlx::query("DELETE FROM time_entries WHERE local_uuid = ?")
        .bind(local_uuid)
        .execute(store.pool())
        .await?;
    Ok(())
}

async fn push_delete(store: &Store, client: &SolidtimeClient, row: &QueueRow) -> Result<()> {
    let payload: DeletePayload = serde_json::from_str(&row.payload)?;
    client.delete_time_entry(&payload.solidtime_id).await?;

    let entries = Entries::new(store.clone());
    sqlx::query("DELETE FROM time_entries WHERE local_uuid = ?")
        .bind(&payload.local_uuid)
        .execute(store.pool())
        .await?;
    let _ = entries;
    Ok(())
}
