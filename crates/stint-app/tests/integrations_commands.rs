//! Integration tests for `commands/integrations.rs`.
//!
//! Drives the `get_api_integration_state` and `set_api_enabled` Tauri
//! commands via `tauri::test::mock_builder()`. Side effects are asserted by
//! reading the `Settings` table directly via stint-core — the same path the
//! production binary would read.

mod common;

use stint_app::commands::integrations::{get_api_integration_state, set_api_enabled};
use stint_core::config::{
    Settings, DEFAULT_API_HOST, KEY_API_ENABLED, KEY_API_HOST, KEY_API_PORT,
};
use tauri::Manager;

#[tokio::test(flavor = "multi_thread")]
async fn get_api_integration_state_defaults_disabled_on_fresh_store() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();
    let state = handle.state();

    let view = get_api_integration_state(state).await.expect("read state");
    assert!(!view.enabled, "fresh store: api disabled");
    assert_eq!(view.host, DEFAULT_API_HOST);
    assert!(view.port.is_none());
    assert!(view.base_url.is_none());
    assert!(!view.bound_this_session);
}

#[tokio::test(flavor = "multi_thread")]
async fn set_api_enabled_persists_to_settings_and_round_trips() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    let view = set_api_enabled(handle.state(), true)
        .await
        .expect("enable api");
    assert!(view.enabled);

    // Direct settings read.
    let settings = Settings::new((*ctx.store).clone());
    let raw = settings
        .get(KEY_API_ENABLED)
        .await
        .expect("get setting")
        .expect("value present");
    assert_eq!(raw, "true");

    // Flip back off.
    let view = set_api_enabled(handle.state(), false)
        .await
        .expect("disable api");
    assert!(!view.enabled);
    let raw = settings
        .get(KEY_API_ENABLED)
        .await
        .expect("get setting")
        .expect("value present");
    assert_eq!(raw, "false");
}

#[tokio::test(flavor = "multi_thread")]
async fn get_api_integration_state_surfaces_persisted_host_port_and_base_url() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    // Seed host + port directly, mirroring what http::maybe_spawn would
    // persist after a successful bind.
    let settings = Settings::new((*ctx.store).clone());
    settings
        .set(KEY_API_ENABLED, "true")
        .await
        .expect("set enabled");
    settings
        .set(KEY_API_HOST, "127.0.0.1")
        .await
        .expect("set host");
    settings
        .set(KEY_API_PORT, "47921")
        .await
        .expect("set port");

    let view = get_api_integration_state(handle.state())
        .await
        .expect("read state");
    assert!(view.enabled);
    assert_eq!(view.host, "127.0.0.1");
    assert_eq!(view.port, Some(47921));
    assert_eq!(view.base_url.as_deref(), Some("http://127.0.0.1:47921"));
    // No bind happened in this test, so bound_this_session is still false.
    assert!(!view.bound_this_session);
}
