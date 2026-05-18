mod common;

use stint_core::store::entries::Entries;
use stint_core::store::queue::Queue;
use stint_core::store::running::RunningTimer;
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

    timer.start(StartArgs {
        description: "a".into(), project_id: None, task_id: None, source: "cli".into(),
    }).await.unwrap();

    let result = timer.start(StartArgs {
        description: "b".into(), project_id: None, task_id: None, source: "cli".into(),
    }).await;

    assert!(matches!(result, Err(stint_core::Error::Invariant(_))));
}

#[tokio::test]
async fn stop_sets_end_clears_running_and_enqueues_update() {
    let env = common::setup().await;
    let timer = TimerService::new(env.store.clone());

    let id = timer.start(StartArgs {
        description: "x".into(), project_id: None, task_id: None, source: "cli".into(),
    }).await.unwrap();

    // small sleep so end > start in seconds resolution
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

    timer.stop().await.unwrap();

    let row = Entries::new(env.store.clone()).get(&id).await.unwrap().unwrap();
    assert!(row.end_at.is_some());
    assert_eq!(row.sync_state, "pending_create");

    assert!(RunningTimer::new(env.store.clone()).get().await.unwrap().is_none());

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
