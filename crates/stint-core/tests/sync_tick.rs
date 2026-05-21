mod common;

use stint_core::config::Settings;
use stint_core::solidtime::SolidtimeClient;
use stint_core::store::reference::Reference;
use stint_core::sync::tick;
use stint_core::timer::{StartArgs, TimerService};
use stint_core::Error;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn enqueue_one_create(env: &common::TestEnv) {
    Settings::new(env.store.clone())
        .set("solidtime.member_id", "m-1")
        .await
        .unwrap();
    TimerService::new(env.store.clone())
        .start(StartArgs {
            description: "x".into(),
            project_id: None,
            task_id: None,
            billable: false,
            source: "cli".into(),
            start_at: None,
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn tick_count_zero_drains_queue_and_refreshes_reference() {
    let env = common::setup().await;
    enqueue_one_create(&env).await;

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": { "id": "remote-1", "description": "x", "start": "2026-05-20T09:00:00Z" }
        })))
        .mount(&server)
        .await;
    // Seed a project on the server so we can observe that refresh happened.
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/projects"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{ "id": "p1", "name": "Tet", "color": null, "client_id": null, "archived": false }]
        })))
        .mount(&server)
        .await;
    // `refresh_reference_data` fetches clients first; without a mock the
    // 404 short-circuits the refresh before projects are upserted.
    for endpoint in ["clients", "tasks", "tags"] {
        Mock::given(method("GET"))
            .and(path(format!("/api/v1/organizations/org-1/{endpoint}")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "data": [] })),
            )
            .mount(&server)
            .await;
    }

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");

    tick(&env.store, &client, 0).await.unwrap();

    // Queue drained: nothing left due.
    let due = stint_core::store::queue::Queue::new(env.store.clone())
        .take_due(10)
        .await
        .unwrap();
    assert!(due.is_empty(), "queue should be drained on tick");

    // Reference refresh ran: projects table has the seeded row.
    let projects = Reference::new(env.store.clone())
        .list_projects()
        .await
        .unwrap();
    assert_eq!(
        projects.len(),
        1,
        "reference data should be refreshed on tick 0"
    );
    assert_eq!(projects[0].id, "p1");
}

#[tokio::test]
async fn tick_count_one_drains_queue_without_refreshing_reference() {
    let env = common::setup().await;
    enqueue_one_create(&env).await;

    let server = MockServer::start().await;
    // Only the entry POST is mocked. If refresh fired, the unmocked GETs
    // would 404 — but refresh-failure is swallowed by tick, so the test
    // observes the absence via projects table being empty.
    Mock::given(method("POST"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": { "id": "remote-1", "description": "x", "start": "2026-05-20T09:00:00Z" }
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");

    tick(&env.store, &client, 1).await.unwrap();

    let due = stint_core::store::queue::Queue::new(env.store.clone())
        .take_due(10)
        .await
        .unwrap();
    assert!(due.is_empty(), "queue should be drained");

    let projects = Reference::new(env.store.clone())
        .list_projects()
        .await
        .unwrap();
    assert!(
        projects.is_empty(),
        "reference data should NOT be refreshed on non-multiple-of-15 ticks",
    );
}

#[tokio::test]
async fn tick_propagates_auth_failure_from_drain() {
    let env = common::setup().await;
    enqueue_one_create(&env).await;

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");

    let err = tick(&env.store, &client, 0).await.unwrap_err();
    assert!(matches!(err, Error::SolidtimeAuth));
}

#[tokio::test]
async fn tick_swallows_reference_refresh_failure() {
    let env = common::setup().await;
    enqueue_one_create(&env).await;

    let server = MockServer::start().await;
    // Drain succeeds, but reference endpoints all 500.
    Mock::given(method("POST"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": { "id": "remote-1", "description": "x", "start": "2026-05-20T09:00:00Z" }
        })))
        .mount(&server)
        .await;
    for endpoint in ["projects", "tasks", "tags"] {
        Mock::given(method("GET"))
            .and(path(format!("/api/v1/organizations/org-1/{endpoint}")))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
    }

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");

    // tick should NOT propagate the refresh error — it's logged + swallowed
    // so a transient ref-data hiccup doesn't kill the sync worker.
    tick(&env.store, &client, 0).await.unwrap();

    let due = stint_core::store::queue::Queue::new(env.store.clone())
        .take_due(10)
        .await
        .unwrap();
    assert!(
        due.is_empty(),
        "queue still drained even when refresh failed"
    );
}
