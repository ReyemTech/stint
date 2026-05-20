mod common;

use stint_core::store::entries::{Entries, RemoteEntryUpsert};

#[tokio::test]
async fn create_from_remote_inserts_synced_row_with_solidtime_id() {
    let env = common::setup().await;
    let entries = Entries::new(env.store.clone());
    let local_uuid = entries
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "remote-1".into(),
            description: "from server".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T10:00:00Z".into(),
            end_at: Some("2026-05-20T11:00:00Z".into()),
            billable: false,
            updated_at: "2026-05-20T11:00:01Z".into(),
        })
        .await
        .unwrap();
    let row = entries.get(&local_uuid).await.unwrap().unwrap();
    assert_eq!(row.solidtime_id.as_deref(), Some("remote-1"));
    assert_eq!(row.sync_state, "synced");
    assert_eq!(row.source, "solidtime");
}

#[tokio::test]
async fn get_by_solidtime_id_finds_existing_row() {
    let env = common::setup().await;
    let entries = Entries::new(env.store.clone());
    entries
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "remote-2".into(),
            description: "x".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T10:00:00Z".into(),
            end_at: Some("2026-05-20T11:00:00Z".into()),
            billable: false,
            updated_at: "2026-05-20T11:00:01Z".into(),
        })
        .await
        .unwrap();
    let row = entries.get_by_solidtime_id("remote-2").await.unwrap();
    assert!(row.is_some());
    assert_eq!(row.unwrap().description, "x");
    let missing = entries.get_by_solidtime_id("no-such").await.unwrap();
    assert!(missing.is_none());
}

#[tokio::test]
async fn update_from_remote_overwrites_fields_for_synced_row() {
    let env = common::setup().await;
    let entries = Entries::new(env.store.clone());
    entries
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "remote-3".into(),
            description: "old".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T10:00:00Z".into(),
            end_at: Some("2026-05-20T11:00:00Z".into()),
            billable: false,
            updated_at: "2026-05-20T11:00:01Z".into(),
        })
        .await
        .unwrap();
    let changed = entries
        .update_from_remote(
            "remote-3",
            RemoteEntryUpsert {
                solidtime_id: "remote-3".into(),
                description: "new".into(),
                project_id: Some("p-1".into()),
                task_id: None,
                start_at: "2026-05-20T10:00:00Z".into(),
                end_at: Some("2026-05-20T11:30:00Z".into()),
                billable: true,
                updated_at: "2026-05-20T11:30:01Z".into(),
            },
        )
        .await
        .unwrap();
    assert!(changed);
    let row = entries.get_by_solidtime_id("remote-3").await.unwrap().unwrap();
    assert_eq!(row.description, "new");
    assert_eq!(row.project_id.as_deref(), Some("p-1"));
    assert_eq!(row.end_at.as_deref(), Some("2026-05-20T11:30:00Z"));
    assert_eq!(row.billable, 1);
    assert_eq!(row.sync_state, "synced");
}

#[tokio::test]
async fn update_from_remote_skips_pending_local_mutations() {
    let env = common::setup().await;
    let entries = Entries::new(env.store.clone());
    let local_uuid = entries
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "remote-4".into(),
            description: "synced".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T10:00:00Z".into(),
            end_at: Some("2026-05-20T11:00:00Z".into()),
            billable: false,
            updated_at: "2026-05-20T11:00:01Z".into(),
        })
        .await
        .unwrap();
    // Local user edited the description; row is now `dirty`.
    entries
        .update_description(&local_uuid, "local edit")
        .await
        .unwrap();

    let changed = entries
        .update_from_remote(
            "remote-4",
            RemoteEntryUpsert {
                solidtime_id: "remote-4".into(),
                description: "remote edit".into(),
                project_id: None,
                task_id: None,
                start_at: "2026-05-20T10:00:00Z".into(),
                end_at: Some("2026-05-20T11:00:00Z".into()),
                billable: false,
                updated_at: "2026-05-20T12:00:00Z".into(),
            },
        )
        .await
        .unwrap();
    assert!(!changed, "should not overwrite local pending mutation");
    let row = entries.get(&local_uuid).await.unwrap().unwrap();
    assert_eq!(row.description, "local edit");
    assert_eq!(row.sync_state, "dirty");
}

#[tokio::test]
async fn hard_delete_by_solidtime_id_removes_row() {
    let env = common::setup().await;
    let entries = Entries::new(env.store.clone());
    entries
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "remote-5".into(),
            description: "x".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T10:00:00Z".into(),
            end_at: Some("2026-05-20T11:00:00Z".into()),
            billable: false,
            updated_at: "2026-05-20T11:00:01Z".into(),
        })
        .await
        .unwrap();
    let removed = entries.hard_delete_by_solidtime_id("remote-5").await.unwrap();
    assert!(removed);
    assert!(entries.get_by_solidtime_id("remote-5").await.unwrap().is_none());
}

#[tokio::test]
async fn list_synced_in_window_returns_only_window_and_synced() {
    let env = common::setup().await;
    let entries = Entries::new(env.store.clone());
    // In window, synced.
    entries
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "in-window".into(),
            description: "x".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T10:00:00Z".into(),
            end_at: Some("2026-05-20T11:00:00Z".into()),
            billable: false,
            updated_at: "2026-05-20T11:00:01Z".into(),
        })
        .await
        .unwrap();
    // Out of window.
    entries
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "out-of-window".into(),
            description: "x".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-01T10:00:00Z".into(),
            end_at: Some("2026-05-01T11:00:00Z".into()),
            billable: false,
            updated_at: "2026-05-01T11:00:01Z".into(),
        })
        .await
        .unwrap();
    let rows = entries
        .list_synced_in_window("2026-05-20T00:00:00Z", "2026-05-21T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].solidtime_id.as_deref(), Some("in-window"));
}
