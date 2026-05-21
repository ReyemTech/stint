mod common;

use stint_core::store::entries::{Entries, NewCompletedEntry};
use stint_core::store::queue::Queue;
use stint_core::store::running::RunningTimer;
use stint_core::Error;
use stint_core::timer::{StartArgs, TimerService};

#[tokio::test]
async fn start_creates_entry_sets_running_and_enqueues_sync() {
    let env = common::setup().await;
    let timer = TimerService::new(env.store.clone());

    let id = timer
        .start(StartArgs {
            description: "writing tests".into(),
            project_id: None,
            task_id: None,
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();

    let entries = Entries::new(env.store.clone());
    let row = entries.get(&id).await.unwrap().expect("entry exists");
    assert_eq!(row.description, "writing tests");
    assert!(row.end_at.is_none());
    assert_eq!(row.sync_state, "pending_create");

    let running = RunningTimer::new(env.store.clone());
    assert_eq!(running.get().await.unwrap().unwrap().local_uuid, id);

    let queue = Queue::new(env.store.clone());
    let due = queue.take_due(10).await.unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].op, "create_entry");
}

#[tokio::test]
async fn start_while_already_running_returns_invariant_error() {
    let env = common::setup().await;
    let timer = TimerService::new(env.store.clone());

    timer
        .start(StartArgs {
            description: "a".into(),
            project_id: None,
            task_id: None,
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();

    let result = timer
        .start(StartArgs {
            description: "b".into(),
            project_id: None,
            task_id: None,
            billable: false,
            source: "cli".into(),
        })
        .await;

    assert!(matches!(result, Err(stint_core::Error::Invariant(_))));
}

#[tokio::test]
async fn stop_sets_end_clears_running_and_enqueues_update() {
    let env = common::setup().await;
    let timer = TimerService::new(env.store.clone());

    let id = timer
        .start(StartArgs {
            description: "x".into(),
            project_id: None,
            task_id: None,
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();

    // small sleep so end > start in seconds resolution
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

    timer.stop().await.unwrap();

    let row = Entries::new(env.store.clone())
        .get(&id)
        .await
        .unwrap()
        .unwrap();
    assert!(row.end_at.is_some());
    assert_eq!(row.sync_state, "pending_create");

    assert!(RunningTimer::new(env.store.clone())
        .get()
        .await
        .unwrap()
        .is_none());

    // sync_queue: create_entry only (still pending_create, so update is folded in via set_end)
    let due = Queue::new(env.store.clone()).take_due(10).await.unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].op, "create_entry");
}

#[tokio::test]
async fn stop_with_no_timer_running_errors() {
    let env = common::setup().await;
    let timer = TimerService::new(env.store.clone());

    let err = timer.stop().await.unwrap_err();
    assert!(matches!(err, stint_core::Error::Invariant(_)));
}

#[tokio::test]
async fn delete_synced_entry_enqueues_delete_op() {
    let env = common::setup().await;
    let timer = TimerService::new(env.store.clone());

    let id = timer
        .start(StartArgs {
            description: "x".into(),
            project_id: None,
            task_id: None,
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();
    timer.stop().await.unwrap();

    let entries = Entries::new(env.store.clone());
    entries.mark_synced(&id, "remote-id").await.unwrap();

    timer.delete(&id).await.unwrap();

    let queue = Queue::new(env.store.clone());
    let due = queue.take_due(10).await.unwrap();
    let delete_ops: Vec<_> = due.iter().filter(|r| r.op == "delete_entry").collect();
    assert_eq!(delete_ops.len(), 1);
}

#[tokio::test]
async fn start_rolls_back_entry_if_running_timer_already_claimed() {
    // Reproduce the TOCTOU race: another writer claims running_timer
    // between TimerService::start's check and its set. With the atomic
    // try_claim_with + transaction, the just-inserted entry must roll
    // back and no orphan time_entries row should remain.
    let env = common::setup().await;

    // Pre-seed running_timer pointing at a synthetic entry (mimics what
    // a concurrent pull adoption would have left behind).
    let entries = Entries::new(env.store.clone());
    let adopted_uuid = entries
        .create(stint_core::store::entries::NewTimeEntry {
            description: "remote-adopted".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T20:00:00Z".into(),
            billable: false,
            source: "solidtime".into(),
        })
        .await
        .unwrap();
    RunningTimer::new(env.store.clone())
        .set(&adopted_uuid)
        .await
        .unwrap();

    let before = entries
        .list_between("2026-01-01T00:00:00Z", "2099-01-01T00:00:00Z")
        .await
        .unwrap()
        .len();

    let timer = TimerService::new(env.store.clone());
    let err = timer
        .start(StartArgs {
            description: "user's typed entry".into(),
            project_id: None,
            task_id: None,
            billable: false,
            source: "cli".into(),
        })
        .await
        .expect_err("expected `already running` error");
    assert!(
        err.to_string().contains("a timer is already running"),
        "wrong error: {err}"
    );

    // No new time_entries row leaked.
    let after = entries
        .list_between("2026-01-01T00:00:00Z", "2099-01-01T00:00:00Z")
        .await
        .unwrap()
        .len();
    assert_eq!(after, before, "tx should have rolled back the inserted row");

    // running_timer still points at the originally-adopted entry.
    let r = RunningTimer::new(env.store.clone())
        .get()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.local_uuid, adopted_uuid);

    // No stray queue ops.
    assert!(Queue::new(env.store.clone())
        .take_due(10)
        .await
        .unwrap()
        .is_empty());
}

async fn create_completed_entry(
    env: &common::TestEnv,
    description: &str,
    project_id: Option<&str>,
    billable: bool,
) -> String {
    Entries::new(env.store.clone())
        .create_completed(NewCompletedEntry {
            description: description.into(),
            project_id: project_id.map(str::to_owned),
            task_id: None,
            start_at: "2026-05-20T20:00:00Z".into(),
            end_at: "2026-05-20T21:00:00Z".into(),
            billable,
            source: "cli".into(),
            source_event_id: None,
        })
        .await
        .unwrap()
}

async fn create_synced_entry(env: &common::TestEnv) -> String {
    let id = create_completed_entry(env, "synced entry", Some("project-a"), false).await;
    Entries::new(env.store.clone())
        .mark_synced(&id, "remote-id")
        .await
        .unwrap();
    id
}

#[tokio::test]
async fn delete_local_only_entry_removes_row_without_delete_queue_op() {
    let env = common::setup().await;
    let timer = TimerService::new(env.store.clone());
    let id = create_completed_entry(&env, "offline entry", None, false).await;

    timer.delete(&id).await.unwrap();

    assert!(Entries::new(env.store.clone()).get(&id).await.unwrap().is_none());
    assert!(Queue::new(env.store.clone())
        .take_due(10)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn delete_missing_entry_returns_not_found() {
    let env = common::setup().await;
    let timer = TimerService::new(env.store.clone());

    let err = timer.delete("missing-entry").await.unwrap_err();
    assert!(matches!(err, Error::NotFound(ref msg) if msg == "entry missing-entry"));
}

#[tokio::test]
async fn update_description_on_synced_entry_marks_dirty_and_enqueues_update() {
    let env = common::setup().await;
    let timer = TimerService::new(env.store.clone());
    let id = create_synced_entry(&env).await;

    timer.update_description(&id, "reworded").await.unwrap();

    let row = Entries::new(env.store.clone()).get(&id).await.unwrap().unwrap();
    assert_eq!(row.description, "reworded");
    assert_eq!(row.sync_state, "dirty");

    let due = Queue::new(env.store.clone()).take_due(10).await.unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].op, "update_entry");
    assert_eq!(due[0].entry_uuid.as_deref(), Some(id.as_str()));
    assert!(due[0].payload.contains("\"description\":\"reworded\""));
}

#[tokio::test]
async fn synced_mutations_enqueue_updates_and_pending_create_mutations_do_not() {
    let env = common::setup().await;
    let timer = TimerService::new(env.store.clone());
    let synced_id = create_synced_entry(&env).await;

    timer.set_project(&synced_id, Some("project-b")).await.unwrap();
    timer.set_billable(&synced_id, true).await.unwrap();

    let synced_row = Entries::new(env.store.clone())
        .get(&synced_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(synced_row.project_id.as_deref(), Some("project-b"));
    assert_eq!(synced_row.billable, 1);
    assert_eq!(synced_row.sync_state, "dirty");

    let due = Queue::new(env.store.clone()).take_due(10).await.unwrap();
    let update_ops: Vec<_> = due.iter().filter(|row| row.op == "update_entry").collect();
    assert_eq!(update_ops.len(), 2);

    let pending_env = common::setup().await;
    let pending_timer = TimerService::new(pending_env.store.clone());
    let pending_id = create_completed_entry(&pending_env, "draft", None, false).await;

    pending_timer
        .update_description(&pending_id, "draft updated")
        .await
        .unwrap();
    pending_timer
        .set_project(&pending_id, Some("project-c"))
        .await
        .unwrap();
    pending_timer.set_billable(&pending_id, true).await.unwrap();

    let pending_row = Entries::new(pending_env.store.clone())
        .get(&pending_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pending_row.description, "draft updated");
    assert_eq!(pending_row.project_id.as_deref(), Some("project-c"));
    assert_eq!(pending_row.billable, 1);
    assert_eq!(pending_row.sync_state, "pending_create");
    assert!(Queue::new(pending_env.store.clone())
        .take_due(10)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn missing_entry_mutations_return_not_found() {
    let env = common::setup().await;
    let timer = TimerService::new(env.store.clone());

    let err = timer
        .update_description("missing-entry", "new description")
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound(ref msg) if msg == "entry missing-entry"));

    let err = timer
        .set_project("missing-entry", Some("project-z"))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound(ref msg) if msg == "entry missing-entry"));

    let err = timer
        .set_billable("missing-entry", true)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound(ref msg) if msg == "entry missing-entry"));
}
