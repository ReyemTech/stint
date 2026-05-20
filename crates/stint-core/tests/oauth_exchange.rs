use stint_core::oauth::client::{OAuthClient, OAuthConfig};
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn cfg(base: &str) -> OAuthConfig {
    OAuthConfig {
        authorize_url: format!("{base}/oauth/authorize"),
        token_url: format!("{base}/oauth/token"),
        client_id: "stint-desktop".into(),
        client_secret: None,
        redirect_uri: "http://127.0.0.1:54321/callback".into(),
        scopes: vec!["read".into(), "create".into()],
        extra_authorize_params: vec![],
    }
}

#[tokio::test]
async fn exchange_code_posts_form_and_parses_tokens() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=authorization_code"))
        .and(body_string_contains("code=test-code"))
        .and(body_string_contains("code_verifier=test-verifier"))
        .and(body_string_contains("client_id=stint-desktop"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "token_type": "Bearer",
            "expires_in": 3600,
            "access_token": "access-1",
            "refresh_token": "refresh-1",
            "scope": "read create"
        })))
        .mount(&server)
        .await;

    let client = OAuthClient::new(cfg(&server.uri()));
    let tokens = client
        .exchange_code("test-code", "test-verifier")
        .await
        .unwrap();
    assert_eq!(tokens.access_token, "access-1");
    assert_eq!(tokens.refresh_token.as_deref(), Some("refresh-1"));
    assert_eq!(tokens.scope.as_deref(), Some("read create"));
}

#[tokio::test]
async fn exchange_code_surfaces_oauth_server_on_4xx() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": "invalid_grant",
            "error_description": "Authorization code has expired"
        })))
        .mount(&server)
        .await;

    let client = OAuthClient::new(cfg(&server.uri()));
    let err = client
        .exchange_code("test-code", "test-verifier")
        .await
        .unwrap_err();
    match err {
        stint_core::Error::OAuthServer(msg) => {
            assert!(msg.contains("invalid_grant"), "got {msg}");
            assert!(msg.contains("expired"), "got {msg}");
        }
        e => panic!("expected OAuthServer, got {e:?}"),
    }
}
