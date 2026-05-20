mod common;

use stint_core::config::Settings;
use stint_core::solidtime::SolidtimeClient;
use stint_core::store::entries::Entries;
use stint_core::store::running::RunningTimer;
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
async fn adopts_remote_running_when_local_idle() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "remote-running",
                "description": "started in web",
                "start": "2026-05-20T16:30:00Z",
                "end": null,
                "billable": false,
                "updated_at": "2026-05-20T16:30:00Z"
            }]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");

    // Precondition: no running timer locally.
    let running = RunningTimer::new(env.store.clone());
    assert!(running.get().await.unwrap().is_none());

    let report = pull(&env.store, &client, Trigger::OnStartup).await.unwrap();

    // Adoption succeeded.
    assert!(report.adopted.is_some());
    assert!(report.conflict.is_none());
    let adopted_uuid = report.adopted.unwrap();

    let entries = Entries::new(env.store.clone());
    let row = entries.get(&adopted_uuid).await.unwrap().unwrap();
    assert_eq!(row.solidtime_id.as_deref(), Some("remote-running"));
    assert_eq!(row.source, "solidtime");
    assert_eq!(row.sync_state, "synced");
    assert!(row.end_at.is_none());

    let running_row = running.get().await.unwrap().unwrap();
    assert_eq!(running_row.local_uuid, adopted_uuid);
}
