//! Integration tests for `commands/sync.rs`.

mod common;

use stint_app::commands::sync::sync_now;
use stint_app::commands::timer::{start_timer, stop_timer, StartTimerArgs};
use stint_core::config::secrets::Secrets;
use stint_core::config::Settings;
use stint_core::store::entries::Entries;
use tauri::Manager;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn seed_solidtime_config(
    store: &std::sync::Arc<stint_core::store::Store>,
    url: &str,
) {
    let settings = Settings::new((**store).clone());
    settings.set("solidtime.url", url).await.unwrap();
    settings.set("solidtime.org", "org-1").await.unwrap();
    settings.set("solidtime.member_id", "m-1").await.unwrap();
    Secrets::default()
        .set("solidtime.token", "test-token")
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_now_drains_one_pending_create_and_marks_entry_synced() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    // Enqueue a create_entry op by starting and stopping a timer.
    let id = start_timer(
        handle.clone(),
        handle.state(),
        StartTimerArgs {
            description: "deep work".into(),
            project_id: None,
            task_id: None,
            billable: false,
        },
    )
    .await
    .unwrap();
    stop_timer(handle.clone(), handle.state()).await.unwrap();

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": { "id": "remote-1", "description": "deep work", "start": "2026-05-20T09:00:00Z" }
        })))
        .expect(1)
        .mount(&server)
        .await;

    seed_solidtime_config(&ctx.store, &server.uri()).await;

    let n = sync_now(handle.clone(), handle.state()).await.unwrap();
    assert_eq!(n, 1);

    let row = Entries::new((*ctx.store).clone())
        .get(&id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.sync_state, "synced");
    assert_eq!(row.solidtime_id.as_deref(), Some("remote-1"));
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_now_errors_when_solidtime_url_missing() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();
    let err = sync_now(handle.clone(), handle.state()).await.unwrap_err();
    assert!(err.message.contains("solidtime.url"), "got: {}", err.message);
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_now_with_empty_queue_returns_zero() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    let server = MockServer::start().await;
    seed_solidtime_config(&ctx.store, &server.uri()).await;

    // Nothing enqueued — drain returns 0 and no Solidtime calls happen.
    let n = sync_now(handle.clone(), handle.state()).await.unwrap();
    assert_eq!(n, 0);
}
