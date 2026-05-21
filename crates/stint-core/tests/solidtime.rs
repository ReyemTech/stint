mod common;

use stint_core::solidtime::SolidtimeClient;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn fake_server() -> MockServer {
    MockServer::start().await
}

#[tokio::test]
async fn test_connection_calls_users_me_with_bearer_token() {
    let server = fake_server().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/users/me"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": { "id": "user-1", "email": "me@example.com" }
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "test-token");
    let me = client.test_connection().await.unwrap();
    assert_eq!(me.id, "user-1");
}

#[tokio::test]
async fn test_connection_maps_401_to_auth_error() {
    let server = fake_server().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/users/me"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "bad-token");
    let err = client.test_connection().await.unwrap_err();
    assert!(matches!(err, stint_core::Error::SolidtimeAuth));
}

#[tokio::test]
async fn test_connection_maps_non_401_failures_to_solidtime_error() {
    let server = fake_server().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/users/me"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(502).set_body_string("upstream exploded"))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "test-token");
    let err = client.test_connection().await.unwrap_err();
    match err {
        stint_core::Error::Solidtime { status, body } => {
            assert_eq!(status, 502);
            assert_eq!(body, "upstream exploded");
        }
        other => panic!("expected Solidtime error, got {other:?}"),
    }
}

#[tokio::test]
async fn list_projects_returns_remote_rows() {
    let server = fake_server().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/projects"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                { "id": "p1", "name": "Tet", "color": "#aaa", "client_id": null, "archived": false },
                { "id": "p2", "name": "Reyem", "color": null, "client_id": null, "archived": false }
            ]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let projects = client.list_projects().await.unwrap();
    assert_eq!(projects.len(), 2);
    assert_eq!(projects[0].id, "p1");
}

#[tokio::test]
async fn list_tasks_maps_non_success_errors() {
    let server = fake_server().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/tasks"))
        .and(header("Authorization", "Bearer t"))
        .respond_with(ResponseTemplate::new(500).set_body_string("server sad"))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let err = client.list_tasks().await.unwrap_err();
    match err {
        stint_core::Error::Solidtime { status, body } => {
            assert_eq!(status, 500);
            assert_eq!(body, "server sad");
        }
        other => panic!("expected Solidtime error, got {other:?}"),
    }
}

#[tokio::test]
async fn list_memberships_maps_401_to_auth_error() {
    let server = fake_server().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/users/me/memberships"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t");
    let err = client.list_memberships().await.unwrap_err();
    assert!(matches!(err, stint_core::Error::SolidtimeAuth));
}

#[tokio::test]
async fn list_projects_requires_org() {
    let server = fake_server().await;
    let client = SolidtimeClient::with_api_token(&server.uri(), "t"); // no org
    let err = client.list_projects().await.unwrap_err();
    assert!(matches!(err, stint_core::Error::MissingConfig(_)));
}

use stint_core::solidtime::dto::CreateEntryRequest;
use wiremock::matchers::body_partial_json;

#[tokio::test]
async fn create_time_entry_posts_and_returns_id() {
    let server = fake_server().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .and(body_partial_json(
            serde_json::json!({ "description": "test" }),
        ))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": { "id": "remote-1", "description": "test", "start": "2026-05-17T09:00:00Z" }
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let req = CreateEntryRequest {
        member_id: "m-1",
        description: "test",
        project_id: None,
        task_id: None,
        start: "2026-05-17T09:00:00Z",
        end: None,
        billable: false,
    };
    let remote = client.create_time_entry(&req).await.unwrap();
    assert_eq!(remote.id, "remote-1");
}

#[tokio::test]
async fn create_time_entry_maps_non_success_failures() {
    let server = fake_server().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .and(header("Authorization", "Bearer t"))
        .respond_with(ResponseTemplate::new(422).set_body_string("bad payload"))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let req = CreateEntryRequest {
        member_id: "m-1",
        description: "test",
        project_id: None,
        task_id: None,
        start: "2026-05-17T09:00:00Z",
        end: None,
        billable: false,
    };
    let err = client.create_time_entry(&req).await.unwrap_err();
    match err {
        stint_core::Error::Solidtime { status, body } => {
            assert_eq!(status, 422);
            assert_eq!(body, "bad payload");
        }
        other => panic!("expected Solidtime error, got {other:?}"),
    }
}

#[tokio::test]
async fn update_time_entry_maps_401_to_auth_error() {
    let server = fake_server().await;

    Mock::given(method("PUT"))
        .and(path("/api/v1/organizations/org-1/time-entries/remote-1"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let req = CreateEntryRequest {
        member_id: "m-1",
        description: "test",
        project_id: None,
        task_id: None,
        start: "2026-05-17T09:00:00Z",
        end: None,
        billable: false,
    };
    let err = client
        .update_time_entry("remote-1", &req)
        .await
        .unwrap_err();
    assert!(matches!(err, stint_core::Error::SolidtimeAuth));
}

#[tokio::test]
async fn delete_time_entry_handles_204() {
    let server = fake_server().await;
    Mock::given(method("DELETE"))
        .and(path("/api/v1/organizations/org-1/time-entries/remote-1"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    client.delete_time_entry("remote-1").await.unwrap();
}

#[tokio::test]
async fn delete_time_entry_treats_404_as_success() {
    let server = fake_server().await;
    Mock::given(method("DELETE"))
        .and(path("/api/v1/organizations/org-1/time-entries/gone"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    client.delete_time_entry("gone").await.unwrap();
}

#[tokio::test]
async fn delete_time_entry_maps_401_to_auth_error() {
    let server = fake_server().await;
    Mock::given(method("DELETE"))
        .and(path("/api/v1/organizations/org-1/time-entries/remote-1"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let err = client.delete_time_entry("remote-1").await.unwrap_err();
    assert!(matches!(err, stint_core::Error::SolidtimeAuth));
}

#[tokio::test]
async fn delete_time_entry_surfaces_non_404_failures() {
    let server = fake_server().await;
    Mock::given(method("DELETE"))
        .and(path("/api/v1/organizations/org-1/time-entries/remote-1"))
        .and(header("Authorization", "Bearer t"))
        .respond_with(ResponseTemplate::new(503).set_body_string("temporarily unavailable"))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let err = client.delete_time_entry("remote-1").await.unwrap_err();
    match err {
        stint_core::Error::Solidtime { status, body } => {
            assert_eq!(status, 503);
            assert_eq!(body, "temporarily unavailable");
        }
        other => panic!("expected Solidtime error, got {other:?}"),
    }
}
