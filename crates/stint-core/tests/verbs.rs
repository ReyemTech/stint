mod common;

use stint_core::store::entries::Entries;
use stint_core::verbs::{self, EntryFilter, StartParams};

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

#[tokio::test]
async fn stop_sets_end_at_and_returns_completed_view() {
    let env = common::setup().await;
    let store = &env.store;

    let started = verbs::start(
        store,
        StartParams {
            description: "task A".into(),
            project_id: None,
            task_id: None,
            billable: false,
            start_at: None,
            source: "test".into(),
        },
    )
    .await
    .unwrap();

    let stopped = verbs::stop(store).await.expect("stop should succeed");
    assert_eq!(stopped.local_uuid, started.local_uuid);
    assert!(stopped.end_at.is_some(), "end_at must be set after stop");
}

#[tokio::test]
async fn stop_errors_when_no_timer_running() {
    let env = common::setup().await;
    let store = &env.store;
    let err = verbs::stop(store).await.unwrap_err();
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("no") || msg.contains("not"),
        "error should indicate no timer running, got: {msg}"
    );
}

#[tokio::test]
async fn current_returns_none_when_idle() {
    let env = common::setup().await;
    let store = &env.store;
    let view = verbs::current(store).await.unwrap();
    assert!(view.is_none());
}

#[tokio::test]
async fn current_returns_running_entry() {
    let env = common::setup().await;
    let store = &env.store;
    let started = verbs::start(
        store,
        StartParams {
            description: "task B".into(),
            project_id: None,
            task_id: None,
            billable: false,
            start_at: None,
            source: "test".into(),
        },
    )
    .await
    .unwrap();
    let view = verbs::current(store).await.unwrap().unwrap();
    assert_eq!(view.local_uuid, started.local_uuid);
    assert!(view.end_at.is_none());
}

#[tokio::test]
async fn list_entries_returns_all_by_default() {
    let env = common::setup().await;
    let store = &env.store;

    for desc in ["a", "b", "c"] {
        verbs::start(
            store,
            StartParams {
                description: desc.into(),
                project_id: None,
                task_id: None,
                billable: false,
                start_at: None,
                source: "test".into(),
            },
        )
        .await
        .unwrap();
        verbs::stop(store).await.unwrap();
    }

    let entries = verbs::list_entries(store, EntryFilter::default())
        .await
        .unwrap();
    assert_eq!(
        entries.len(),
        3,
        "got: {:?}",
        entries.iter().map(|e| &e.description).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn list_entries_respects_limit() {
    let env = common::setup().await;
    let store = &env.store;
    for desc in ["a", "b", "c", "d"] {
        verbs::start(
            store,
            StartParams {
                description: desc.into(),
                project_id: None,
                task_id: None,
                billable: false,
                start_at: None,
                source: "test".into(),
            },
        )
        .await
        .unwrap();
        verbs::stop(store).await.unwrap();
    }

    let entries = verbs::list_entries(
        store,
        EntryFilter {
            limit: Some(2),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(entries.len(), 2);
}

#[tokio::test]
async fn list_entries_filters_by_project() {
    let env = common::setup().await;
    let store = &env.store;

    // Two entries with project, one without.
    for (desc, pid) in [
        ("with-p", Some("p-1".to_string())),
        ("no-p", None),
        ("also-with-p", Some("p-1".to_string())),
    ] {
        verbs::start(
            store,
            StartParams {
                description: desc.into(),
                project_id: pid,
                task_id: None,
                billable: false,
                start_at: None,
                source: "test".into(),
            },
        )
        .await
        .unwrap();
        verbs::stop(store).await.unwrap();
    }

    let filtered = verbs::list_entries(
        store,
        EntryFilter {
            project_id: Some("p-1".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    assert_eq!(filtered.len(), 2);
    assert!(filtered
        .iter()
        .all(|e| e.project_id.as_deref() == Some("p-1")));
}
