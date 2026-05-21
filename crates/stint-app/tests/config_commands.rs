//! Integration tests for `commands/config.rs`.
//!
//! Skips `oauth_solidtime_start` — it launches the system browser. The
//! status / logout / set / show / test paths cover the non-interactive
//! surface.

mod common;

use stint_app::commands::config::{
    config_set, config_show, config_test, oauth_solidtime_logout, oauth_solidtime_status,
    solidtime_url,
};
use stint_core::config::secrets::Secrets;
use stint_core::config::Settings;
use tauri::Manager;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test(flavor = "multi_thread")]
async fn config_show_marks_secret_keys_separately() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    // Seed a non-secret setting and a secret value.
    Settings::new((*ctx.store).clone())
        .set("solidtime.url", "https://example.com")
        .await
        .unwrap();
    Secrets::default().set("solidtime.token", "tok-abc").unwrap();

    let entries = config_show(handle.state()).await.unwrap();
    let url = entries.iter().find(|e| e.key == "solidtime.url").unwrap();
    assert_eq!(url.value.as_deref(), Some("https://example.com"));
    assert!(!url.is_secret);

    let tok = entries
        .iter()
        .find(|e| e.key == "solidtime.token")
        .expect("token entry present");
    assert!(tok.is_secret);
    assert!(tok.present, "token should be marked present");
    assert!(tok.value.is_none(), "secret value should not leak in show");
}

#[tokio::test(flavor = "multi_thread")]
async fn config_set_routes_secrets_to_keychain_and_settings_to_db() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    config_set(
        handle.state(),
        "solidtime.url".into(),
        "https://example.com".into(),
    )
    .await
    .unwrap();
    config_set(
        handle.state(),
        "solidtime.token".into(),
        "tok-xyz".into(),
    )
    .await
    .unwrap();

    // Non-secret persisted to the settings table.
    let settings = Settings::new((*ctx.store).clone());
    assert_eq!(
        settings.get("solidtime.url").await.unwrap().as_deref(),
        Some("https://example.com")
    );
    // Secret persisted to Keychain (synthetic prefix via common::make_app).
    assert_eq!(
        Secrets::default().get("solidtime.token").unwrap().as_deref(),
        Some("tok-xyz")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn solidtime_url_returns_none_when_unset_and_trims_trailing_slash() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    let result = solidtime_url(handle.state()).await.unwrap();
    assert!(result.is_none());

    Settings::new((*ctx.store).clone())
        .set("solidtime.url", "https://example.com/")
        .await
        .unwrap();

    let result = solidtime_url(handle.state()).await.unwrap();
    assert_eq!(result.as_deref(), Some("https://example.com"));
}

#[tokio::test(flavor = "multi_thread")]
async fn config_test_returns_user_email_on_successful_connection() {
    let ctx = common::make_app().await;
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/users/me"))
        .and(header("Authorization", "Bearer tok-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": { "id": "user-1", "email": "me@example.com" }
        })))
        .mount(&server)
        .await;

    Settings::new((*ctx.store).clone())
        .set("solidtime.url", &server.uri())
        .await
        .unwrap();
    Secrets::default().set("solidtime.token", "tok-abc").unwrap();

    let handle = ctx.handle();
    let who = config_test(handle.state()).await.unwrap();
    assert_eq!(who, "me@example.com");
}

#[tokio::test(flavor = "multi_thread")]
async fn config_test_errors_when_url_unset() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();
    let err = config_test(handle.state()).await.unwrap_err();
    assert!(err.message.contains("solidtime.url"), "got: {}", err.message);
}

#[tokio::test(flavor = "multi_thread")]
async fn oauth_solidtime_status_reflects_api_token_mode() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    // The synthetic prefix is shared across tests in this binary; explicitly
    // clear the token so this assertion isn't polluted by prior tests.
    Secrets::default().delete("solidtime.token").unwrap();

    let s = oauth_solidtime_status(handle.state()).await.unwrap();
    let mode = serde_json::to_value(&s).unwrap();
    assert_eq!(mode["mode"], "api_token");
    assert_eq!(mode["signed_in"], false);

    // Set a token → signed_in true.
    Secrets::default().set("solidtime.token", "tok").unwrap();
    let s = oauth_solidtime_status(handle.state()).await.unwrap();
    let json = serde_json::to_value(&s).unwrap();
    assert_eq!(json["mode"], "api_token");
    assert_eq!(json["signed_in"], true);

    // Clean up so subsequent tests start without this token set.
    Secrets::default().delete("solidtime.token").unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn oauth_solidtime_logout_is_idempotent_when_no_blob_exists() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();
    // No oauth blob to delete — must not error.
    oauth_solidtime_logout(handle.state()).await.unwrap();
}
