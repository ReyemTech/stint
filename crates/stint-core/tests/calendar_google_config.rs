use stint_core::calendar::google::config::{
    google_oauth_config, google_oauth_config_with_client_id, GOOGLE_CALENDAR_READONLY_SCOPE,
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
fn google_oauth_config_honours_env_override() {
    // The build-time constant is consulted when no env var is set; with
    // the env var, the env value wins for tests.
    std::env::set_var(
        "STINT_GOOGLE_CLIENT_ID",
        "override-client.apps.googleusercontent.com",
    );
    let cfg = google_oauth_config();
    assert_eq!(cfg.client_id, "override-client.apps.googleusercontent.com");
    std::env::remove_var("STINT_GOOGLE_CLIENT_ID");
}
