use chrono::Utc;
use stint_core::oauth::client::{OAuthClient, OAuthConfig};
use stint_core::oauth::tokens::TokenSet;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn cfg(base: &str) -> OAuthConfig {
    OAuthConfig {
        authorize_url: format!("{base}/oauth/authorize"),
        token_url: format!("{base}/oauth/token"),
        client_id: "stint-desktop".into(),
        redirect_uri: "http://127.0.0.1:54321/callback".into(),
        scopes: vec!["read".into()],
    }
}

#[tokio::test]
async fn refresh_posts_form_and_returns_new_token_set() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=refresh_token"))
        .and(body_string_contains("refresh_token=old-refresh"))
        .and(body_string_contains("client_id=stint-desktop"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "token_type": "Bearer",
            "expires_in": 3600,
            "access_token": "access-2",
            "refresh_token": "new-refresh",
            "scope": "read"
        })))
        .mount(&server)
        .await;

    let client = OAuthClient::new(cfg(&server.uri()));
    let prior = TokenSet::from_response(
        "access-1".into(),
        Some("old-refresh".into()),
        60,
        None,
        Utc::now(),
    );
    let refreshed = client.refresh_tokens(&prior).await.unwrap();
    assert_eq!(refreshed.access_token, "access-2");
    assert_eq!(refreshed.refresh_token.as_deref(), Some("new-refresh"));
}

#[tokio::test]
async fn refresh_returns_oauth_refresh_failed_on_invalid_grant() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": "invalid_grant",
            "error_description": "Refresh token expired"
        })))
        .mount(&server)
        .await;

    let client = OAuthClient::new(cfg(&server.uri()));
    let prior = TokenSet::from_response("a".into(), Some("expired-r".into()), 0, None, Utc::now());
    let err = client.refresh_tokens(&prior).await.unwrap_err();
    assert!(
        matches!(err, stint_core::Error::OAuthRefreshFailed),
        "got {err:?}"
    );
}
