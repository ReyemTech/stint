mod common;

use stint_core::config::Settings;
use stint_core::solidtime::SolidtimeClient;
use stint_core::store::entries::{Entries, RemoteEntryUpsert};
use stint_core::sync::pull::{pull, Trigger};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn configure(env: &common::TestEnv, server_uri: &str) {
    let s = Settings::new(env.store.clone());
    s.set("solidtime.url", server_uri).await.unwrap();
    s.set("solidtime.org", "org-1").await.unwrap();
    s.set("solidtime.member_id", "m-1").await.unwrap();
}

#[tokio::test]
async fn inserts_new_remote_entries() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {
                    "id": "remote-a",
                    "description": "task a",
                    "start": "2026-05-20T10:00:00Z",
                    "end": "2026-05-20T11:00:00Z",
                    "billable": false,
                    "updated_at": "2026-05-20T11:00:00Z"
                },
                {
                    "id": "remote-b",
                    "description": "task b",
                    "start": "2026-05-20T11:30:00Z",
                    "end": "2026-05-20T12:00:00Z",
                    "billable": true,
                    "updated_at": "2026-05-20T12:00:00Z"
                }
            ]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::Manual).await.unwrap();
    assert_eq!(report.inserted, 2);
    assert_eq!(report.updated, 0);

    let entries = Entries::new(env.store.clone());
    let a = entries
        .get_by_solidtime_id("remote-a")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(a.description, "task a");
    assert_eq!(a.sync_state, "synced");
    assert_eq!(a.source, "solidtime");
    let b = entries
        .get_by_solidtime_id("remote-b")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(b.description, "task b");
    assert_eq!(b.billable, 1);
}

#[tokio::test]
async fn updates_existing_row_when_remote_is_newer() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    // Pre-seed a local synced row with an older updated_at.
    Entries::new(env.store.clone())
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "remote-c".into(),
            description: "old".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T10:00:00Z".into(),
            end_at: Some("2026-05-20T11:00:00Z".into()),
            billable: false,
            updated_at: "2026-05-20T11:00:00Z".into(),
        })
        .await
        .unwrap();

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "remote-c",
                "description": "newer description",
                "start": "2026-05-20T10:00:00Z",
                "end": "2026-05-20T11:00:00Z",
                "billable": true,
                "updated_at": "2026-05-20T12:00:00Z"
            }]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::Manual).await.unwrap();
    assert_eq!(report.inserted, 0);
    assert_eq!(report.updated, 1);

    let entries = Entries::new(env.store.clone());
    let row = entries
        .get_by_solidtime_id("remote-c")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.description, "newer description");
    assert_eq!(row.billable, 1);
}

#[tokio::test]
async fn skips_when_local_is_pending() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    let entries = Entries::new(env.store.clone());
    let local_uuid = entries
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "remote-d".into(),
            description: "synced".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T10:00:00Z".into(),
            end_at: Some("2026-05-20T11:00:00Z".into()),
            billable: false,
            updated_at: "2026-05-20T11:00:00Z".into(),
        })
        .await
        .unwrap();
    // Local edit → row flips to `dirty`.
    entries
        .update_description(&local_uuid, "local edit")
        .await
        .unwrap();

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "remote-d",
                "description": "remote edit",
                "start": "2026-05-20T10:00:00Z",
                "end": "2026-05-20T11:00:00Z",
                "billable": false,
                "updated_at": "2026-05-20T12:00:00Z"
            }]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::Manual).await.unwrap();
    assert_eq!(report.updated, 0);
    let row = entries.get(&local_uuid).await.unwrap().unwrap();
    assert_eq!(
        row.description, "local edit",
        "must not overwrite local pending change"
    );
}

#[tokio::test]
async fn noop_when_local_matches_remote_exactly() {
    // Same fields → no update, regardless of timestamps. Previously this
    // relied on an updated_at comparison, but Solidtime omits updated_at
    // on the list endpoint so we field-compare instead.
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    Entries::new(env.store.clone())
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "remote-e".into(),
            description: "same".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T10:00:00Z".into(),
            end_at: Some("2026-05-20T11:00:00Z".into()),
            billable: false,
            updated_at: "2026-05-20T11:00:00Z".into(),
        })
        .await
        .unwrap();

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "remote-e",
                "description": "same",
                "start": "2026-05-20T10:00:00Z",
                "end": "2026-05-20T11:00:00Z",
                "billable": false
            }]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::Manual).await.unwrap();
    assert_eq!(report.updated, 0);
}

#[tokio::test]
async fn updates_when_remote_omits_updated_at_but_end_at_differs() {
    // Regression: Solidtime's list endpoint does not include `updated_at`.
    // The previous comparison (remote.updated_at > local.updated_at, with
    // a fallback to remote.start) made this skip → local timer kept
    // showing "running" even after the entry was stopped on Solidtime.
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    // Local synced row mirrors a remote that's still "running" — end_at None.
    Entries::new(env.store.clone())
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "stopped-externally".into(),
            description: "test 3".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T20:48:00Z".into(),
            end_at: None,
            billable: false,
            // local updated_at AFTER remote.start — the old comparison
            // (with start as fallback) would say "remote is older" → skip.
            updated_at: "2026-05-20T20:48:46Z".into(),
        })
        .await
        .unwrap();

    // Remote response: same id, end_at now set, NO updated_at field.
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "stopped-externally",
                "description": "test 3",
                "start": "2026-05-20T20:48:00Z",
                "end": "2026-05-20T20:55:00Z",
                "billable": false
            }]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::Manual).await.unwrap();
    assert_eq!(report.updated, 1);

    let row = Entries::new(env.store.clone())
        .get_by_solidtime_id("stopped-externally")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.end_at.as_deref(), Some("2026-05-20T20:55:00Z"));
}

#[tokio::test]
async fn rollback_on_partial_failure_leaves_no_rows() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    // Construct a payload that succeeds partway then fails:
    //   1. A *completed* entry "fresh-a" reconcile_history would
    //      INSERT (no prior row).
    //   2. A *running* entry "collide" whose id matches a pre-seeded
    //      synced row. reconcile_running runs FIRST and INSERTs via
    //      create_from_remote_with — that violates UNIQUE(solidtime_id).
    //
    // If `pull()` is wrapped in a single transaction, the failed
    // running adopt must roll back. The pre-seeded row stays intact;
    // because reconcile_running runs first, history never gets to
    // touch "fresh-a", so it remains absent. (The negative assertion
    // still catches the regression where the tx is committed early
    // or where reconcile_history is allowed to run after a failure.)
    Entries::new(env.store.clone())
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "collide".into(),
            description: "preexisting".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-19T08:00:00Z".into(),
            end_at: Some("2026-05-19T09:00:00Z".into()),
            billable: false,
            updated_at: "2026-05-19T09:00:00Z".into(),
        })
        .await
        .unwrap();

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {
                    "id": "fresh-a",
                    "description": "would-be-inserted",
                    "start": "2026-05-20T11:30:00Z",
                    "end": "2026-05-20T12:00:00Z",
                    "billable": false,
                    "updated_at": "2026-05-20T12:00:00Z"
                },
                {
                    "id": "collide",
                    "description": "remote running",
                    "start": "2026-05-20T10:00:00Z",
                    "end": null,
                    "billable": false,
                    "updated_at": "2026-05-20T10:05:00Z"
                }
            ]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let result = pull(&env.store, &client, Trigger::Manual).await;
    assert!(result.is_err(), "expected UNIQUE violation on collide");

    // Pre-seeded row untouched; nothing leaked from the failed pull.
    let entries = Entries::new(env.store.clone());
    let pre = entries
        .get_by_solidtime_id("collide")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pre.description, "preexisting");
    assert!(entries
        .get_by_solidtime_id("fresh-a")
        .await
        .unwrap()
        .is_none());
}
