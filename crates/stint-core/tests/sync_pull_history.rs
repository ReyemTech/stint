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
    let a = entries.get_by_solidtime_id("remote-a").await.unwrap().unwrap();
    assert_eq!(a.description, "task a");
    assert_eq!(a.sync_state, "synced");
    assert_eq!(a.source, "solidtime");
    let b = entries.get_by_solidtime_id("remote-b").await.unwrap().unwrap();
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
    let row = entries.get_by_solidtime_id("remote-c").await.unwrap().unwrap();
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
    entries.update_description(&local_uuid, "local edit").await.unwrap();

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
    assert_eq!(row.description, "local edit", "must not overwrite local pending change");
}

#[tokio::test]
async fn noop_when_local_is_newer() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    Entries::new(env.store.clone())
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "remote-e".into(),
            description: "local-most-recent".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T10:00:00Z".into(),
            end_at: Some("2026-05-20T11:00:00Z".into()),
            billable: false,
            updated_at: "2026-05-20T13:00:00Z".into(),
        })
        .await
        .unwrap();

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "remote-e",
                "description": "remote-older",
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
    let row = Entries::new(env.store.clone())
        .get_by_solidtime_id("remote-e")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.description, "local-most-recent");
}
