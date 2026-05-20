use chrono::{Duration as ChronoDuration, Utc};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use stint_core::oauth::client::{OAuthClient, OAuthConfig};
use stint_core::oauth::tokens::TokenSet;
use stint_core::solidtime::auth::{OAuthTokenProvider, TokenProvider};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn oauth_client_for(server: &MockServer) -> OAuthClient {
    OAuthClient::new(OAuthConfig {
        authorize_url: format!("{}/oauth/authorize", server.uri()),
        token_url: format!("{}/oauth/token", server.uri()),
        client_id: "stint-desktop".into(),
        redirect_uri: "http://127.0.0.1:0/callback".into(),
        scopes: vec!["read".into()],
        extra_authorize_params: vec![],
    })
}

#[tokio::test]
async fn returns_cached_access_token_when_not_expired() {
    let server = MockServer::start().await;
    // No mock for /oauth/token — if it gets hit, the test fails with 404.
    let client = oauth_client_for(&server);
    let saved = Arc::new(Mutex::new(None));
    let saved_clone = saved.clone();
    let persist = move |t: &TokenSet| {
        *saved_clone.lock().unwrap() = Some(t.clone());
        Ok(())
    };

    let token_set = TokenSet::from_response(
        "fresh-access".into(),
        Some("r".into()),
        3600,
        None,
        Utc::now(),
    );
    let provider = OAuthTokenProvider::new(client, token_set, Box::new(persist));
    let got = provider.access_token().await.unwrap();
    assert_eq!(got, "fresh-access");
    assert!(
        saved.lock().unwrap().is_none(),
        "no persist should have happened — token was fresh"
    );
}

#[tokio::test]
async fn refreshes_and_persists_when_expired() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "token_type": "Bearer",
            "expires_in": 3600,
            "access_token": "refreshed-access",
            "refresh_token": "refreshed-refresh",
            "scope": "read"
        })))
        .mount(&server)
        .await;
    let client = oauth_client_for(&server);
    let saved = Arc::new(Mutex::new(None));
    let saved_clone = saved.clone();
    let persist = move |t: &TokenSet| {
        *saved_clone.lock().unwrap() = Some(t.clone());
        Ok(())
    };

    // Expired 5 min ago.
    let token_set = TokenSet {
        access_token: "stale".into(),
        refresh_token: Some("old-refresh".into()),
        expires_at: Utc::now() - ChronoDuration::minutes(5),
        scope: None,
    };
    let provider = OAuthTokenProvider::new(client, token_set, Box::new(persist));
    let got = provider.access_token().await.unwrap();
    assert_eq!(got, "refreshed-access");

    let saved = saved.lock().unwrap();
    let saved = saved.as_ref().expect("should have persisted");
    assert_eq!(saved.access_token, "refreshed-access");
    assert_eq!(saved.refresh_token.as_deref(), Some("refreshed-refresh"));
}

#[tokio::test]
async fn surfaces_oauth_refresh_failed_when_server_rejects_refresh() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": "invalid_grant"
        })))
        .mount(&server)
        .await;
    let client = oauth_client_for(&server);
    let persist = |_: &TokenSet| Ok(());
    let token_set = TokenSet {
        access_token: "stale".into(),
        refresh_token: Some("expired-refresh".into()),
        expires_at: Utc::now() - ChronoDuration::minutes(5),
        scope: None,
    };
    let provider = OAuthTokenProvider::new(client, token_set, Box::new(persist));
    let err = provider.access_token().await.unwrap_err();
    assert!(
        matches!(err, stint_core::Error::OAuthRefreshFailed),
        "got {err:?}"
    );
    // Sleep briefly to allow any deferred wiremock asserts to flush.
    tokio::time::sleep(Duration::from_millis(10)).await;
}
