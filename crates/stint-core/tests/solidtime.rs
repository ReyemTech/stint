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

    let client = SolidtimeClient::new(&server.uri(), "test-token");
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

    let client = SolidtimeClient::new(&server.uri(), "bad-token");
    let err = client.test_connection().await.unwrap_err();
    assert!(matches!(err, stint_core::Error::SolidtimeAuth));
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

    let client = SolidtimeClient::new(&server.uri(), "t").with_org("org-1");
    let projects = client.list_projects().await.unwrap();
    assert_eq!(projects.len(), 2);
    assert_eq!(projects[0].id, "p1");
}

#[tokio::test]
async fn list_projects_requires_org() {
    let server = fake_server().await;
    let client = SolidtimeClient::new(&server.uri(), "t"); // no org
    let err = client.list_projects().await.unwrap_err();
    assert!(matches!(err, stint_core::Error::MissingConfig(_)));
}
