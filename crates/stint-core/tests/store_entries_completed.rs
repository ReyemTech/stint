mod common;

use stint_core::store::entries::{Entries, NewCompletedEntry};

#[tokio::test]
async fn create_completed_persists_all_fields() {
    let env = common::setup().await;
    let e = Entries::new(env.store.clone());

    let uuid = e
        .create_completed(NewCompletedEntry {
            description: "Sprint review".into(),
            project_id: Some("p-1".into()),
            task_id: None,
            start_at: "2026-05-19T14:00:00Z".into(),
            end_at: "2026-05-19T15:00:00Z".into(),
            billable: true,
            source: "calendar".into(),
            source_event_id: Some("acc-1:evt-1:2026-05-19T14:00:00Z".into()),
        })
        .await
        .unwrap();

    let row = e.get(&uuid).await.unwrap().expect("entry persisted");
    assert_eq!(row.description, "Sprint review");
    assert_eq!(row.start_at, "2026-05-19T14:00:00Z");
    assert_eq!(row.end_at.as_deref(), Some("2026-05-19T15:00:00Z"));
    assert_eq!(row.source, "calendar");
    assert_eq!(
        row.source_event_id.as_deref(),
        Some("acc-1:evt-1:2026-05-19T14:00:00Z")
    );
    assert_eq!(row.billable, 1);
    assert_eq!(row.sync_state, "pending_create");
}

#[tokio::test]
async fn create_completed_returns_unique_uuids() {
    let env = common::setup().await;
    let e = Entries::new(env.store.clone());
    let mk = || NewCompletedEntry {
        description: "x".into(),
        project_id: None,
        task_id: None,
        start_at: "2026-05-19T09:00:00Z".into(),
        end_at: "2026-05-19T09:30:00Z".into(),
        billable: false,
        source: "calendar".into(),
        source_event_id: None,
    };
    let a = e.create_completed(mk()).await.unwrap();
    let b = e.create_completed(mk()).await.unwrap();
    assert_ne!(a, b);
}
