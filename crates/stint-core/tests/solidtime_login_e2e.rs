use std::time::Duration;
use stint_core::oauth::client::{OAuthClient, OAuthConfig};
use stint_core::solidtime::auth::login_interactive;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn interactive_login_completes_against_mock_authz_server() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "token_type": "Bearer",
            "expires_in": 3600,
            "access_token": "first-access",
            "refresh_token": "first-refresh",
            "scope": "read create update delete"
        })))
        .mount(&server)
        .await;

    let client = OAuthClient::new(OAuthConfig {
        authorize_url: format!("{}/oauth/authorize", server.uri()),
        token_url: format!("{}/oauth/token", server.uri()),
        client_id: "stint-desktop".into(),
        redirect_uri: "http://127.0.0.1:0/callback".into(),
        scopes: vec![
            "read".into(),
            "create".into(),
            "update".into(),
            "delete".into(),
        ],
        extra_authorize_params: vec![],
    });

    // Simulate the browser hitting the callback in the background.
    let browser_simulator = |authorize_url: String| {
        tokio::spawn(async move {
            // Parse the redirect_uri + state from the authorize URL.
            let parsed = url::Url::parse(&authorize_url).unwrap();
            let mut state = None;
            let mut redirect = None;
            for (k, v) in parsed.query_pairs() {
                match k.as_ref() {
                    "state" => state = Some(v.into_owned()),
                    "redirect_uri" => redirect = Some(v.into_owned()),
                    _ => {}
                }
            }
            let redirect = redirect.expect("authorize URL has redirect_uri");
            let state = state.expect("authorize URL has state");
            // Wait briefly so the loopback server has a chance to start accepting.
            tokio::time::sleep(Duration::from_millis(50)).await;
            let callback = format!("{redirect}?code=ok-code&state={state}");
            let _ = reqwest::get(&callback).await;
        });
    };

    let tokens = login_interactive(
        &client,
        Duration::from_secs(10),
        "Solidtime",
        browser_simulator,
    )
    .await
    .unwrap();
    assert_eq!(tokens.access_token, "first-access");
    assert_eq!(tokens.refresh_token.as_deref(), Some("first-refresh"));
}
