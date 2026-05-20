mod common;

use stint_core::config::Settings;
use stint_core::solidtime::SolidtimeClient;
use stint_core::store::entries::Entries;
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
