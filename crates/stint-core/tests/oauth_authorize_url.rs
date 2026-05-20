use stint_core::oauth::client::{OAuthClient, OAuthConfig};

fn cfg() -> OAuthConfig {
    OAuthConfig {
        authorize_url: "https://time.example.com/oauth/authorize".into(),
        token_url: "https://time.example.com/oauth/token".into(),
        client_id: "stint-desktop".into(),
        redirect_uri: "http://127.0.0.1:54321/callback".into(),
        scopes: vec![
            "read".into(),
            "create".into(),
            "update".into(),
            "delete".into(),
        ],
        extra_authorize_params: vec![],
    }
}

#[test]
fn authorize_url_includes_pkce_and_state() {
    let client = OAuthClient::new(cfg());
    let prepared = client.prepare_authorize();
    let url = prepared.authorize_url.as_str();
    assert!(url.starts_with("https://time.example.com/oauth/authorize?"));
    assert!(url.contains("response_type=code"));
    assert!(url.contains("client_id=stint-desktop"));
    assert!(url.contains("redirect_uri=http"));
    assert!(url.contains("scope=read"));
    assert!(url.contains("code_challenge_method=S256"));
    assert!(url.contains("code_challenge="));
    assert!(url.contains("state="));
    assert!(!prepared.code_verifier.is_empty());
    assert!(!prepared.state.is_empty());
}

#[test]
fn two_prepares_produce_distinct_verifiers_and_states() {
    let client = OAuthClient::new(cfg());
    let a = client.prepare_authorize();
    let b = client.prepare_authorize();
    assert_ne!(a.code_verifier, b.code_verifier);
    assert_ne!(a.state, b.state);
}

#[test]
fn authorize_url_appends_extra_params_in_order() {
    use stint_core::oauth::client::{OAuthClient, OAuthConfig};

    let cfg = OAuthConfig {
        authorize_url: "https://accounts.google.com/o/oauth2/v2/auth".into(),
        token_url: "https://oauth2.googleapis.com/token".into(),
        client_id: "fake-id".into(),
        redirect_uri: "http://127.0.0.1:0/callback".into(),
        scopes: vec!["https://www.googleapis.com/auth/calendar.readonly".into()],
        extra_authorize_params: vec![
            ("access_type".into(), "offline".into()),
            ("prompt".into(), "consent".into()),
        ],
    };
    let prepared = OAuthClient::new(cfg).prepare_authorize();
    let url = prepared.authorize_url.to_string();
    assert!(url.contains("access_type=offline"), "got {url}");
    assert!(url.contains("prompt=consent"), "got {url}");
    assert!(url.contains("response_type=code"));
    assert!(url.contains("code_challenge_method=S256"));
}
