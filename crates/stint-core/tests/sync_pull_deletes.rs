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
