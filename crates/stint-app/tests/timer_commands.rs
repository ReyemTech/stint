//! Integration tests for `commands/timer.rs` via `tauri::test::mock_builder()`.
//!
//! Each test builds a fresh mock app with a tempdir-backed Store and
//! invokes the `#[tauri::command]` function directly. Side effects are
//! asserted by reading the store back through stint-core's Service API
//! (Entries, RunningTimer, Queue) — the same path the production binary
//! would read.

mod common;

use stint_app::commands::timer::{
    delete_entry, get_running_timer, set_entry_billable, set_entry_project, start_timer,
    stop_timer, update_description, update_entry_times, StartTimerArgs,
};
use stint_core::store::entries::Entries;
use stint_core::store::queue::Queue;
use stint_core::store::running::RunningTimer;
use tauri::Manager;

#[tokio::test(flavor = "multi_thread")]
async fn get_running_timer_returns_none_on_fresh_store() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();
    let state = handle.state();

    let result = get_running_timer(state).await.expect("command succeeds");
    assert!(result.is_none(), "no timer should be running on a fresh DB");
}

#[tokio::test(flavor = "multi_thread")]
async fn start_timer_persists_entry_and_enqueues_create_op() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();
    let state = handle.state();

    let id = start_timer(
        handle.clone(),
        state,
        StartTimerArgs {
            description: "design review".into(),
            project_id: None,
            task_id: None,
            billable: true,
            start_at: None,
        },
    )
    .await
    .expect("start_timer succeeds");
    assert!(!id.is_empty());

    // Entry persisted with pending_create state.
    let entries = Entries::new((*ctx.store).clone());
    let row = entries.get(&id).await.unwrap().expect("entry present");
    assert_eq!(row.description, "design review");
    assert_eq!(row.billable, 1);
    assert_eq!(row.sync_state, "pending_create");
    assert!(row.end_at.is_none());

    // running_timer points to it.
    let running = RunningTimer::new((*ctx.store).clone())
        .get()
        .await
        .unwrap()
        .expect("running_timer set");
    assert_eq!(running.local_uuid, id);

    // Exactly one queue op enqueued.
    let queue = Queue::new((*ctx.store).clone());
    let due = queue.take_due(10).await.unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].op, "create_entry");
}

#[tokio::test(flavor = "multi_thread")]
async fn start_timer_while_running_returns_invariant_error() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();
    let state = handle.state();

    start_timer(
        handle.clone(),
        state,
        StartTimerArgs {
            description: "first".into(),
            project_id: None,
            task_id: None,
            billable: false,
            start_at: None,
        },
    )
    .await
    .expect("first start succeeds");

    // Re-fetch state for the second call (the borrow above is consumed).
    let state = handle.state();
    let err = start_timer(
        handle.clone(),
        state,
        StartTimerArgs {
            description: "second".into(),
            project_id: None,
            task_id: None,
            billable: false,
            start_at: None,
        },
    )
    .await
    .expect_err("second start should fail");
    assert!(
        err.message.contains("already running"),
        "got: {}",
        err.message
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn stop_timer_sets_end_and_clears_running() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    let id = start_timer(
        handle.clone(),
        handle.state(),
        StartTimerArgs {
            description: "x".into(),
            project_id: None,
            task_id: None,
            billable: false,
            start_at: None,
        },
    )
    .await
    .unwrap();

    let stopped_id = stop_timer(handle.clone(), handle.state()).await.unwrap();
    assert_eq!(stopped_id, id);

    let row = Entries::new((*ctx.store).clone())
        .get(&id)
        .await
        .unwrap()
        .unwrap();
    assert!(row.end_at.is_some());
    assert!(RunningTimer::new((*ctx.store).clone())
        .get()
        .await
        .unwrap()
        .is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_entry_on_pending_create_hard_deletes_the_row() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    let id = start_timer(
        handle.clone(),
        handle.state(),
        StartTimerArgs {
            description: "x".into(),
            project_id: None,
            task_id: None,
            billable: false,
            start_at: None,
        },
    )
    .await
    .unwrap();
    stop_timer(handle.clone(), handle.state()).await.unwrap();

    delete_entry(handle.clone(), handle.state(), id.clone())
        .await
        .expect("delete succeeds");

    let row = Entries::new((*ctx.store).clone()).get(&id).await.unwrap();
    assert!(row.is_none(), "pending_create entry should be hard-deleted");
}

#[tokio::test(flavor = "multi_thread")]
async fn update_description_round_trips() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    let id = start_timer(
        handle.clone(),
        handle.state(),
        StartTimerArgs {
            description: "old".into(),
            project_id: None,
            task_id: None,
            billable: false,
            start_at: None,
        },
    )
    .await
    .unwrap();

    update_description(handle.clone(), handle.state(), id.clone(), "new".into())
        .await
        .expect("update succeeds");

    let row = Entries::new((*ctx.store).clone())
        .get(&id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.description, "new");
}

#[tokio::test(flavor = "multi_thread")]
async fn set_entry_project_round_trips() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    let id = start_timer(
        handle.clone(),
        handle.state(),
        StartTimerArgs {
            description: "x".into(),
            project_id: None,
            task_id: None,
            billable: false,
            start_at: None,
        },
    )
    .await
    .unwrap();

    set_entry_project(
        handle.clone(),
        handle.state(),
        id.clone(),
        Some("p-7".into()),
    )
    .await
    .expect("set project succeeds");

    let row = Entries::new((*ctx.store).clone())
        .get(&id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.project_id.as_deref(), Some("p-7"));

    // Clearing the project also round-trips.
    set_entry_project(handle.clone(), handle.state(), id.clone(), None)
        .await
        .unwrap();
    let row = Entries::new((*ctx.store).clone())
        .get(&id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.project_id, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn set_entry_billable_round_trips() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    let id = start_timer(
        handle.clone(),
        handle.state(),
        StartTimerArgs {
            description: "x".into(),
            project_id: None,
            task_id: None,
            billable: false,
            start_at: None,
        },
    )
    .await
    .unwrap();

    set_entry_billable(handle.clone(), handle.state(), id.clone(), true)
        .await
        .expect("set billable succeeds");

    let row = Entries::new((*ctx.store).clone())
        .get(&id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.billable, 1);

    set_entry_billable(handle.clone(), handle.state(), id.clone(), false)
        .await
        .unwrap();
    let row = Entries::new((*ctx.store).clone())
        .get(&id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.billable, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn update_entry_times_updates_both_fields_and_enqueues_update_when_synced() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    // Start + stop to get a completed entry, then mark synced so the
    // command's maybe_enqueue_update sees a "dirty" → enqueue path.
    let id = start_timer(
        handle.clone(),
        handle.state(),
        StartTimerArgs {
            description: "edit me".into(),
            project_id: None,
            task_id: None,
            billable: false,
            start_at: None,
        },
    )
    .await
    .unwrap();
    stop_timer(handle.clone(), handle.state()).await.unwrap();
    let entries = Entries::new((*ctx.store).clone());
    entries.mark_synced(&id, "remote-id").await.unwrap();

    let queue = Queue::new((*ctx.store).clone());
    // take_due is a peek, not a drain — capture the baseline so we can check
    // that update_entry_times added exactly one new op.
    let before = queue.take_due(100).await.unwrap();

    update_entry_times(
        handle.clone(),
        handle.state(),
        id.clone(),
        "2026-05-20T09:00:00Z".into(),
        "2026-05-20T10:00:00Z".into(),
    )
    .await
    .expect("update_entry_times succeeds");

    let row = entries.get(&id).await.unwrap().unwrap();
    assert_eq!(row.start_at, "2026-05-20T09:00:00Z");
    assert_eq!(row.end_at.as_deref(), Some("2026-05-20T10:00:00Z"));
    assert_eq!(row.sync_state, "dirty");

    let after = queue.take_due(100).await.unwrap();
    assert_eq!(after.len(), before.len() + 1);
    assert_eq!(
        after.last().unwrap().op,
        "update_entry",
        "the new op should be update_entry"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn update_entry_times_rejects_end_le_start() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    let id = start_timer(
        handle.clone(),
        handle.state(),
        StartTimerArgs {
            description: "x".into(),
            project_id: None,
            task_id: None,
            billable: false,
            start_at: None,
        },
    )
    .await
    .unwrap();
    stop_timer(handle.clone(), handle.state()).await.unwrap();

    let err = update_entry_times(
        handle.clone(),
        handle.state(),
        id,
        "2026-05-20T11:00:00Z".into(),
        "2026-05-20T10:00:00Z".into(),
    )
    .await
    .expect_err("end < start should be rejected");
    assert!(
        err.message.contains("end must be after start"),
        "got: {}",
        err.message
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn get_running_timer_returns_view_after_start() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    let id = start_timer(
        handle.clone(),
        handle.state(),
        StartTimerArgs {
            description: "deep work".into(),
            project_id: Some("p-1".into()),
            task_id: None,
            billable: true,
            start_at: None,
        },
    )
    .await
    .unwrap();

    let view = get_running_timer(handle.state())
        .await
        .unwrap()
        .expect("running timer present");
    assert_eq!(view.local_uuid, id);
    assert_eq!(view.description, "deep work");
    assert_eq!(view.project_id.as_deref(), Some("p-1"));
    assert!(view.billable);
}
