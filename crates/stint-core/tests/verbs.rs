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
