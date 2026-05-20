use stint_core::calendar::google::config::{
    google_oauth_config_with_client_id, GOOGLE_CALENDAR_READONLY_SCOPE,
};
use stint_core::oauth::client::OAuthClient;

#[test]
fn google_oauth_config_includes_required_endpoints_and_scope() {
    let cfg = google_oauth_config_with_client_id("fake-client.apps.googleusercontent.com");
    assert_eq!(
        cfg.authorize_url,
        "https://accounts.google.com/o/oauth2/v2/auth"
    );
    assert_eq!(cfg.token_url, "https://oauth2.googleapis.com/token");
    assert_eq!(cfg.client_id, "fake-client.apps.googleusercontent.com");
    assert!(cfg
        .scopes
        .iter()
        .any(|s| s == GOOGLE_CALENDAR_READONLY_SCOPE));
}

#[test]
fn google_authorize_url_carries_access_type_offline_and_prompt_consent() {
    let cfg = google_oauth_config_with_client_id("fake-client.apps.googleusercontent.com");
    let prepared = OAuthClient::new(cfg).prepare_authorize();
    let url = prepared.authorize_url.to_string();
    assert!(url.contains("access_type=offline"), "got {url}");
    assert!(url.contains("prompt=consent"), "got {url}");
    assert!(
        url.contains("scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fcalendar.readonly"),
        "got {url}"
    );
}

#[test]
fn is_configured_reflects_compile_time_presence() {
    // The actual value depends on whether STINT_GOOGLE_CLIENT_ID and
    // STINT_GOOGLE_CLIENT_SECRET were set at build time. We can only
    // assert that the function returns a deterministic bool consistent
    // with the constant emptiness.
    let configured = stint_core::calendar::google::config::is_configured();
    let expected = !stint_core::calendar::google::config::GOOGLE_OAUTH_CLIENT_ID.is_empty()
        && !stint_core::calendar::google::config::GOOGLE_OAUTH_CLIENT_SECRET.is_empty();
    assert_eq!(configured, expected);
}

#[test]
fn google_oauth_config_secret_present_when_baked() {
    let cfg = google_oauth_config_with_client_id("fake-client.apps.googleusercontent.com");
    // Reflects compile-time STINT_GOOGLE_CLIENT_SECRET presence.
    if !stint_core::calendar::google::config::GOOGLE_OAUTH_CLIENT_SECRET.is_empty() {
        assert!(cfg.client_secret.is_some());
    } else {
        assert!(cfg.client_secret.is_none());
    }
}
