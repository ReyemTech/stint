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
            let msg = result
                .as_ref()
                .err()
                .map(|e| e.to_string())
                .unwrap_or_default();
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
            // try_adopt_overlapping returns either Ok (adopted) or a
            // diagnostic-enriched Error::Solidtime { status: 400 } which
            // still matches the is_permanent_4xx abandon path — so just
            // propagate it; no need to fall back to the raw `e`.
            try_adopt_overlapping(
                store,
                client,
                &payload.local_uuid,
                &current.start_at,
                &member,
            )
            .await
        }
        Err(e) => Err(e),
    }
}

/// When Solidtime says we're overlapping ourselves, the usual cause is a
/// missed 201 response: we POSTed earlier, Solidtime persisted the entry,
/// and we never recorded the remote id. Look up Solidtime's view in a
/// small window around our start_at and adopt if we find a 1:1 match —
/// the local row becomes synced with the remote id and the queue item
/// succeeds.
async fn try_adopt_overlapping(
    store: &Store,
    client: &SolidtimeClient,
    local_uuid: &str,
    local_start: &str,
    member_id: &str,
) -> Result<()> {
    let parsed = crate::time::parse(local_start)
        .map_err(|_| Error::Invariant("local start_at unparseable".into()))?;
    // Widen the window ±1-2s to absorb clock-rounding and the small
    // discrepancy between our `:ssZ` normalization and whatever Solidtime
    // recorded server-side.
    let from = crate::time::format(&(parsed - chrono::Duration::seconds(1)));
    let to = crate::time::format(&(parsed + chrono::Duration::seconds(2)));
    let remotes = client.list_time_entries(member_id, &from, &to).await?;

    // Normalize both sides — Solidtime may return offset form (+00:00) or
    // fractional seconds depending on history; equality on the raw strings
    // would miss otherwise-identical timestamps.
    let local_canon = crate::time::to_solidtime_z(local_start);
    let matched = remotes
        .iter()
        .find(|r| crate::time::to_solidtime_z(&r.start) == local_canon);
    if let Some(match_) = matched {
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
    // No exact-start match. Solidtime's `start > X AND start < Y` filter
    // can't see a stale remote that's still running from earlier today, so
    // do a second `active=true` query to surface it. Don't auto-adopt
    // (start mismatch could be a legitimately different timer) — just put
    // it in the error so the user can stop it remotely or `sync
    // force-adopt` it.
    let returned: Vec<String> = remotes
        .iter()
        .map(|r| format!("{}@{}", r.id, r.start))
        .collect();
    let active = client
        .list_active_time_entries(member_id)
        .await
        .unwrap_or_default();
    let active_descr: Vec<String> = active
        .iter()
        .map(|r| format!("{} @ {} ({})", r.id, r.start, r.description))
        .collect();

    warn!(
        local = %local_uuid,
        local_start = %local_start,
        window = %format!("[{from}, {to})"),
        in_window = ?returned,
        active_remotes = ?active_descr,
        "adopt-on-overlap: no exact-start match",
    );
    let body = if active.is_empty() {
        format!(
            "overlapping_time_entry: no remote entry at start={local_start} \
             and no active remote timer either — Solidtime rejected for an \
             unknown reason (window {from}..{to} returned {} candidates)",
            returned.len()
        )
    } else {
        format!(
            "overlapping_time_entry: no remote entry at start={local_start} \
             but Solidtime has {} active remote timer(s) blocking the POST: [{}]. \
             Stop the conflicting one in Solidtime, then `stint sync \
             retry-abandoned`. Or `stint sync force-adopt <local_uuid> \
             <remote_id>` to link this local row to one of the actives.",
            active.len(),
            active_descr.join(", "),
        )
    };
    Err(Error::Solidtime { status: 400, body })
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
