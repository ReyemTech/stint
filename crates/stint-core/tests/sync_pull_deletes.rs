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
async fn deletes_local_when_remote_returns_404() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    // Local has a synced row inside the window (manual = 30 day window).
    let now = chrono::Utc::now();
    let start_at = (now - chrono::Duration::hours(1))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let end_at = (now - chrono::Duration::minutes(30))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    Entries::new(env.store.clone())
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "doomed".into(),
            description: "to be deleted".into(),
            project_id: None,
            task_id: None,
            start_at,
            end_at: Some(end_at),
            billable: false,
            updated_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        })
        .await
        .unwrap();

    // List returns nothing (the row "fell out").
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": []})))
        .mount(&server)
        .await;
    // The per-id GET returns 404 → confirms deletion.
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries/doomed"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::Manual).await.unwrap();
    assert_eq!(report.deleted, 1);

    let entries = Entries::new(env.store.clone());
    assert!(entries.get_by_solidtime_id("doomed").await.unwrap().is_none());
}

#[tokio::test]
async fn keeps_local_when_remote_get_returns_200() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    let now = chrono::Utc::now();
    let start_at = (now - chrono::Duration::hours(1))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let end_at = (now - chrono::Duration::minutes(30))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    Entries::new(env.store.clone())
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "still-here".into(),
            description: "alive elsewhere".into(),
            project_id: None,
            task_id: None,
            start_at: start_at.clone(),
            end_at: Some(end_at.clone()),
            billable: false,
            updated_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        })
        .await
        .unwrap();

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": []})))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries/still-here"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "id": "still-here",
                "description": "alive elsewhere",
                "start": start_at,
                "end": end_at,
                "billable": false
            }
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::Manual).await.unwrap();
    assert_eq!(report.deleted, 0);

    assert!(Entries::new(env.store.clone())
        .get_by_solidtime_id("still-here")
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn caps_delete_probes_at_50_per_pull() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    let entries = Entries::new(env.store.clone());
    // 60 local rows in the window — staggered start times so list_synced_in_window
    // returns them in deterministic order. solidtime_ids row-00 .. row-59.
    for i in 0..60 {
        // Spread starts across the last hour. i=0 is oldest, i=59 newest.
        let start_at = (chrono::Utc::now() - chrono::Duration::minutes(60 - i))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let end_at = (chrono::Utc::now() - chrono::Duration::minutes(59 - i))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        entries
            .create_from_remote(RemoteEntryUpsert {
                solidtime_id: format!("row-{i:02}"),
                description: "x".into(),
                project_id: None,
                task_id: None,
                start_at,
                end_at: Some(end_at),
                billable: false,
                updated_at: chrono::Utc::now()
                    .format("%Y-%m-%dT%H:%M:%SZ")
                    .to_string(),
            })
            .await
            .unwrap();
    }

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": []})))
        .mount(&server)
        .await;
    // Any per-id GET returns 404 — but only 50 such requests should fire.
    Mock::given(method("GET"))
        .and(wiremock::matchers::path_regex(
            r"^/api/v1/organizations/org-1/time-entries/row-\d+$",
        ))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::Manual).await.unwrap();
    assert_eq!(report.deleted, 50);

    // Verify exactly 10 rows survived (60 - 50 deleted = 10).
    let remaining = Entries::new(env.store.clone())
        .list_synced_in_window("2026-01-01T00:00:00Z", "2099-01-01T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(remaining.len(), 10);
}
