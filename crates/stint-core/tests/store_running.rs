mod common;

use stint_core::store::entries::{Entries, NewTimeEntry};
use stint_core::store::running::RunningTimer;

#[tokio::test]
async fn set_then_get_running_timer() {
    let env = common::setup().await;
    let entries = Entries::new(env.store.clone());
    let running = RunningTimer::new(env.store.clone());

    let id = entries
        .create(NewTimeEntry {
            description: "x".into(), project_id: None, task_id: None,
            start_at: "2026-05-17T09:00:00Z".into(),
            billable: false, source: "cli".into(),
        })
        .await
        .unwrap();

    running.set(&id).await.unwrap();
    let got = running.get().await.unwrap().expect("running timer set");
    assert_eq!(got.local_uuid, id);
}

#[tokio::test]
async fn clear_removes_running_timer() {
    let env = common::setup().await;
    let entries = Entries::new(env.store.clone());
    let running = RunningTimer::new(env.store.clone());

    let id = entries
        .create(NewTimeEntry {
            description: "x".into(), project_id: None, task_id: None,
            start_at: "2026-05-17T09:00:00Z".into(),
            billable: false, source: "cli".into(),
        })
        .await
        .unwrap();

    running.set(&id).await.unwrap();
    running.clear().await.unwrap();
    assert!(running.get().await.unwrap().is_none());
}

#[tokio::test]
async fn heartbeat_updates_timestamp() {
    let env = common::setup().await;
    let entries = Entries::new(env.store.clone());
    let running = RunningTimer::new(env.store.clone());

    let id = entries
        .create(NewTimeEntry {
            description: "x".into(), project_id: None, task_id: None,
            start_at: "2026-05-17T09:00:00Z".into(),
            billable: false, source: "cli".into(),
        })
        .await
        .unwrap();

    running.set(&id).await.unwrap();
    let first = running.get().await.unwrap().unwrap().heartbeat_at;

    // Allow at least 1s of clock movement.
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    running.heartbeat().await.unwrap();
    let second = running.get().await.unwrap().unwrap().heartbeat_at;
    assert_ne!(first, second);
}
