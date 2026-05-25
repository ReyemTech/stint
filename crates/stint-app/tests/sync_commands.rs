//! Integration tests for `commands/sync.rs`.

mod common;

use stint_app::commands::sync::{get_sync_error_overlaps, list_sync_errors, sync_now};
use stint_app::commands::timer::{start_timer, stop_timer, StartTimerArgs};
use stint_core::config::secrets::Secrets;
use stint_core::config::Settings;
use stint_core::store::entries::Entries;
use stint_core::store::queue::{Queue, QueueOp};
use tauri::Manager;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Find the most recent sync_queue row for a given local entry uuid.
/// Used to drive mark_failed / mark_abandoned. `take_due` is a SELECT
/// despite its name — it does not delete rows.
async fn queue_row_id_for_entry(
    store: &std::sync::Arc<stint_core::store::Store>,
    local_uuid: &str,
) -> i64 {
    let queue = Queue::new((**store).clone());
    let rows = queue.take_due(100).await.expect("take_due ok");
    rows.into_iter()
        .filter(|r| r.entry_uuid.as_deref() == Some(local_uuid))
        .max_by_key(|r| r.id)
        .expect("queue row present")
        .id
}

async fn seed_solidtime_config(store: &std::sync::Arc<stint_core::store::Store>, url: &str) {
    let settings = Settings::new((**store).clone());
    settings.set("solidtime.url", url).await.unwrap();
    settings.set("solidtime.org", "org-1").await.unwrap();
    settings.set("solidtime.member_id", "m-1").await.unwrap();
    Secrets::default()
        .set("solidtime.token", "test-token")
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_now_drains_one_pending_create_and_marks_entry_synced() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    // Enqueue a create_entry op by starting and stopping a timer.
    let view = start_timer(
        handle.clone(),
        handle.state(),
        StartTimerArgs {
            description: "deep work".into(),
            project_id: None,
            task_id: None,
            billable: false,
            start_at: None,
        },
    )
    .await
    .unwrap();
    let id = view.local_uuid.clone();
    stop_timer(handle.clone(), handle.state()).await.unwrap();

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": { "id": "remote-1", "description": "deep work", "start": "2026-05-20T09:00:00Z" }
        })))
        .expect(1)
        .mount(&server)
        .await;

    seed_solidtime_config(&ctx.store, &server.uri()).await;

    let n = sync_now(handle.clone(), handle.state()).await.unwrap();
    assert_eq!(n, 1);

    let row = Entries::new((*ctx.store).clone())
        .get(&id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.sync_state, "synced");
    assert_eq!(row.solidtime_id.as_deref(), Some("remote-1"));
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_now_errors_when_solidtime_url_missing() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();
    let err = sync_now(handle.clone(), handle.state()).await.unwrap_err();
    assert!(
        err.message.contains("solidtime.url"),
        "got: {}",
        err.message
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn list_sync_errors_is_empty_on_fresh_store() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();
    let rows = list_sync_errors(handle.state()).await.unwrap();
    assert!(rows.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn list_sync_errors_surfaces_failed_rows_with_view_fields() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    // Create a real entry, then enqueue + immediately fail an op against
    // it so the LEFT JOIN populates description / start / end on the view.
    let view = start_timer(
        handle.clone(),
        handle.state(),
        StartTimerArgs {
            description: "failing entry".into(),
            project_id: None,
            task_id: None,
            billable: false,
            start_at: None,
        },
    )
    .await
    .unwrap();
    stop_timer(handle.clone(), handle.state()).await.unwrap();

    let queue = Queue::new((*ctx.store).clone());
    // Drain so we know the queue id; then fail it three times to surpass
    // the min_attempts=3 filter inside list_failed_with_entry.
    let entry_json = serde_json::json!({"local_uuid": view.local_uuid}).to_string();
    queue
        .enqueue(QueueOp::UpdateEntry, &entry_json, Some(&view.local_uuid))
        .await
        .unwrap();
    // Pull the row id back out — list_failed_with_entry uses queue.id.
    let id = queue_row_id_for_entry(&ctx.store, &view.local_uuid).await;
    for _ in 0..3 {
        queue.mark_failed(id, "synthetic failure").await.unwrap();
    }

    let errors = list_sync_errors(handle.state()).await.unwrap();
    let found = errors
        .iter()
        .find(|e| e.local_uuid.as_deref() == Some(view.local_uuid.as_str()))
        .expect("our errored row should surface");
    assert_eq!(found.op, "update_entry");
    assert_eq!(found.attempts, 3);
    assert_eq!(found.last_error.as_deref(), Some("synthetic failure"));
    assert_eq!(found.description.as_deref(), Some("failing entry"));
    // mark_failed (not mark_abandoned), so next_try_at is in the near future.
    assert!(!found.abandoned, "transient failure must not be abandoned");
}

#[tokio::test(flavor = "multi_thread")]
async fn list_sync_errors_flags_abandoned_rows() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    let view = start_timer(
        handle.clone(),
        handle.state(),
        StartTimerArgs {
            description: "abandoned entry".into(),
            project_id: None,
            task_id: None,
            billable: false,
            start_at: None,
        },
    )
    .await
    .unwrap();
    stop_timer(handle.clone(), handle.state()).await.unwrap();

    let queue = Queue::new((*ctx.store).clone());
    let entry_json = serde_json::json!({"local_uuid": view.local_uuid}).to_string();
    queue
        .enqueue(QueueOp::UpdateEntry, &entry_json, Some(&view.local_uuid))
        .await
        .unwrap();
    let id = queue_row_id_for_entry(&ctx.store, &view.local_uuid).await;
    // Need attempts ≥ 3 for list_failed_with_entry to pick the row up.
    queue.mark_failed(id, "synthetic").await.unwrap();
    queue.mark_failed(id, "synthetic").await.unwrap();
    queue.mark_abandoned(id, "permanent 422").await.unwrap();

    let errors = list_sync_errors(handle.state()).await.unwrap();
    let found = errors
        .iter()
        .find(|e| e.local_uuid.as_deref() == Some(view.local_uuid.as_str()))
        .expect("abandoned row should surface");
    assert!(
        found.abandoned,
        "row parked >30d in the future must read as abandoned"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn get_sync_error_overlaps_returns_empty_when_entry_missing() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();
    let overlaps = get_sync_error_overlaps(handle.state(), "no-such-uuid".into())
        .await
        .unwrap();
    assert!(overlaps.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn get_sync_error_overlaps_returns_empty_when_solidtime_url_missing() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    let view = start_timer(
        handle.clone(),
        handle.state(),
        StartTimerArgs {
            description: "no-config".into(),
            project_id: None,
            task_id: None,
            billable: false,
            start_at: None,
        },
    )
    .await
    .unwrap();
    stop_timer(handle.clone(), handle.state()).await.unwrap();

    // No solidtime.url set → command short-circuits with empty.
    let overlaps = get_sync_error_overlaps(handle.state(), view.local_uuid)
        .await
        .unwrap();
    assert!(overlaps.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn get_sync_error_overlaps_filters_remote_entries_by_time_range() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    let view = start_timer(
        handle.clone(),
        handle.state(),
        StartTimerArgs {
            description: "overlapping".into(),
            project_id: None,
            task_id: None,
            billable: false,
            // Pick a specific past window so we can mock matching remote rows.
            start_at: Some("2026-05-23T10:00:00Z".into()),
        },
    )
    .await
    .unwrap();
    // Use update to set an explicit end_at, then save.
    stop_timer(handle.clone(), handle.state()).await.unwrap();
    use stint_app::commands::timer::update_entry_times;
    update_entry_times(
        handle.clone(),
        handle.state(),
        view.local_uuid.clone(),
        "2026-05-23T10:00:00Z".into(),
        "2026-05-23T11:00:00Z".into(),
    )
    .await
    .unwrap();

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {
                    "id": "remote-overlap",
                    "description": "I clash",
                    "start": "2026-05-23T10:30:00Z",
                    "end": "2026-05-23T11:30:00Z",
                    "project_id": null,
                    "task_id": null,
                    "billable": false,
                    "user_id": null,
                    "member_id": "m-1"
                },
                {
                    "id": "remote-far",
                    "description": "way before",
                    "start": "2026-05-21T08:00:00Z",
                    "end":   "2026-05-21T09:00:00Z",
                    "project_id": null,
                    "task_id": null,
                    "billable": false,
                    "user_id": null,
                    "member_id": "m-1"
                }
            ]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries/active"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": []})))
        .mount(&server)
        .await;

    seed_solidtime_config(&ctx.store, &server.uri()).await;

    let overlaps = get_sync_error_overlaps(handle.state(), view.local_uuid)
        .await
        .unwrap();
    // Only the truly-overlapping row should survive the time-range filter.
    assert_eq!(overlaps.len(), 1);
    assert_eq!(overlaps[0].id, "remote-overlap");
    assert_eq!(overlaps[0].description, "I clash");
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_now_with_empty_queue_returns_zero() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    let server = MockServer::start().await;
    seed_solidtime_config(&ctx.store, &server.uri()).await;

    // Nothing enqueued — drain returns 0 and no Solidtime calls happen.
    let n = sync_now(handle.clone(), handle.state()).await.unwrap();
    assert_eq!(n, 0);
}
