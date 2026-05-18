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
