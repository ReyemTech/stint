mod common;

use stint_core::store::entries::{Entries, NewTimeEntry};

#[tokio::test]
async fn create_then_get_round_trips() {
    let env = common::setup().await;
    let entries = Entries::new(env.store.clone());

    let id = entries
        .create(NewTimeEntry {
            description: "writing tests".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-17T15:00:00Z".into(),
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();

    let row = entries.get(&id).await.unwrap().expect("row exists");
    assert_eq!(row.description, "writing tests");
    assert_eq!(row.start_at, "2026-05-17T15:00:00Z");
    assert_eq!(row.end_at, None);
    assert_eq!(row.sync_state, "pending_create");
    assert_eq!(row.source, "cli");
    assert!(row.solidtime_id.is_none());
}

#[tokio::test]
async fn list_between_returns_entries_in_range() {
    let env = common::setup().await;
    let entries = Entries::new(env.store.clone());

    for &start in &[
        "2026-05-15T09:00:00Z",
        "2026-05-16T09:00:00Z",
        "2026-05-17T09:00:00Z",
    ] {
        entries
            .create(NewTimeEntry {
                description: "x".into(),
                project_id: None,
                task_id: None,
                start_at: start.into(),
                billable: false,
                source: "cli".into(),
            })
            .await
            .unwrap();
    }

    let rows = entries
        .list_between("2026-05-16T00:00:00Z", "2026-05-17T23:59:59Z")
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
}
