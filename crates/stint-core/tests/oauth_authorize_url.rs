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
