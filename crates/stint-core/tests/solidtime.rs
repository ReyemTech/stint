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
