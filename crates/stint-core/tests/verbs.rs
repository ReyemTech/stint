mod common;

use stint_core::store::entries::Entries;
use stint_core::verbs::{self, StartParams};

#[tokio::test]
async fn start_creates_running_entry_and_returns_view() {
    let env = common::setup().await;

    let view = verbs::start(
        &env.store,
        StartParams {
            description: "writing tests".into(),
            project_id: None,
            task_id: None,
            billable: false,
            start_at: None,
            source: "test".into(),
        },
    )
    .await
    .expect("start should succeed");

    assert_eq!(view.description, "writing tests");
    assert_eq!(view.source, "test");
    assert!(view.end_at.is_none());

    // Persisted in DB
    let entries = Entries::new(env.store.clone());
    let row = entries.get(&view.local_uuid).await.unwrap().unwrap();
    assert_eq!(row.description, "writing tests");
    assert!(row.end_at.is_none());
}

#[tokio::test]
async fn start_errors_when_timer_already_running() {
    let env = common::setup().await;

    verbs::start(
        &env.store,
        StartParams {
            description: "first".into(),
            project_id: None,
            task_id: None,
            billable: false,
            start_at: None,
            source: "test".into(),
        },
    )
    .await
    .expect("first start should succeed");

    let result = verbs::start(
        &env.store,
        StartParams {
            description: "second".into(),
            project_id: None,
            task_id: None,
            billable: false,
            start_at: None,
            source: "test".into(),
        },
    )
    .await;

    assert!(result.is_err(), "second start must error");

    // And no second row persisted: only "first" exists.
    let entries = Entries::new(env.store.clone());
    let all = entries
        .list_between("1970-01-01T00:00:00Z", "9999-01-01T00:00:00Z")
        .await
        .unwrap_or_default();
    let firsts: Vec<_> = all.iter().filter(|r| r.description == "first").collect();
    let seconds: Vec<_> = all.iter().filter(|r| r.description == "second").collect();
    assert_eq!(firsts.len(), 1);
    assert_eq!(
        seconds.len(),
        0,
        "no second entry should have been persisted"
    );
}
