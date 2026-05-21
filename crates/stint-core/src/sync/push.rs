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
use tracing::{debug, info, warn};

#[derive(Debug, Deserialize)]
struct CreatePayload {
    local_uuid: String,
}

#[derive(Debug, Deserialize)]
struct DeletePayload {
    local_uuid: String,
    solidtime_id: String,
}

/// 4xx codes that won't fix themselves: validation, permissions, business
/// rules. Retrying just floods logs without resolving anything. 408 (timeout)
/// and 429 (rate limit) intentionally stay transient.
fn is_permanent_4xx(status: u16) -> bool {
    matches!(status, 400 | 403 | 404 | 409 | 410 | 422)
}

/// Solidtime returns 400 with `{"error":true,"key":"overlapping_time_entry",...}`
/// when the requested timeframe collides with an existing entry — most often
/// because we POSTed earlier, missed the 201 response, and are retrying.
fn is_overlap_error(e: &Error) -> bool {
    matches!(e, Error::Solidtime { status: 400, body } if body.contains("overlapping_time_entry"))
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
        Err(Error::Solidtime { status, .. }) if is_permanent_4xx(*status) => {
            let msg = result.as_ref().err().map(|e| e.to_string()).unwrap_or_default();
            warn!(queue_id = row.id, status = *status, error = %msg, "abandoning queue item — non-recoverable 4xx");
            queue.mark_abandoned(row.id, &msg).await?;
        }
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
    match client.create_time_entry(&req).await {
        Ok(remote) => {
            info!(local = %payload.local_uuid, remote = %remote.id, "create_entry synced");
            entries.mark_synced(&payload.local_uuid, &remote.id).await?;
            Ok(())
        }
        Err(e) if is_overlap_error(&e) => {
            info!(
                local = %payload.local_uuid,
                "create_entry got overlapping_time_entry; checking for adoption candidate",
            );
            try_adopt_overlapping(store, client, &payload.local_uuid, &current.start_at, &member)
                .await
                // If adoption fails, return the original error so the
                // permanent-fail path applies (mark_abandoned).
                .or(Err(e))
        }
        Err(e) => Err(e),
    }
}

/// When Solidtime says we're overlapping ourselves, the usual cause is a
/// missed 201 response: we POSTed earlier, Solidtime persisted the entry,
/// and we never recorded the remote id. Look up Solidtime's view at our
/// start_at and adopt if we find a 1:1 match — the local row becomes
/// synced with the remote id and the queue item succeeds.
async fn try_adopt_overlapping(
    store: &Store,
    client: &SolidtimeClient,
    local_uuid: &str,
    local_start: &str,
    member_id: &str,
) -> Result<()> {
    let window_to = match crate::time::parse(local_start) {
        Ok(t) => crate::time::format(&(t + chrono::Duration::seconds(1))),
        Err(_) => return Err(Error::Invariant("local start_at unparseable".into())),
    };
    let remotes = client.list_time_entries(member_id, local_start, &window_to).await?;
    if let Some(match_) = remotes.iter().find(|r| r.start == local_start) {
        info!(
            local = %local_uuid,
            remote = %match_.id,
            "adopted overlapping remote entry — likely from a missed-response retry",
        );
        Entries::new(store.clone())
            .mark_synced(local_uuid, &match_.id)
            .await?;
        return Ok(());
    }
    warn!(
        local = %local_uuid,
        local_start = %local_start,
        "no matching remote at this start_at — overlap is with a different timer",
    );
    Err(Error::Solidtime {
        status: 400,
        body: format!(
            "overlapping_time_entry: no remote entry at start={local_start} \
             — another timer must be active in Solidtime"
        ),
    })
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
