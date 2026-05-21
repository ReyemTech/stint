//! Integration tests for `commands/pull.rs`.

mod common;

use stint_app::commands::pull::{
    conflict_resolve, pull_now, ConflictActionDto, ConflictResolveArgs,
};
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
async fn pull_now_errors_when_solidtime_url_missing() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();
    let err = pull_now(handle.clone(), handle.state()).await.unwrap_err();
    assert!(err.message.contains("solidtime.url"), "got: {}", err.message);
}

#[tokio::test(flavor = "multi_thread")]
async fn pull_now_errors_when_org_missing() {
    let ctx = common::make_app().await;
    Settings::new((*ctx.store).clone())
        .set("solidtime.url", "https://example.com")
        .await
        .unwrap();
    let handle = ctx.handle();
    let err = pull_now(handle.clone(), handle.state()).await.unwrap_err();
    assert!(err.message.contains("solidtime.org"), "got: {}", err.message);
}

#[tokio::test(flavor = "multi_thread")]
async fn pull_now_returns_zero_changes_on_empty_remote() {
    let ctx = common::make_app().await;
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": []
        })))
        .mount(&server)
        .await;

    seed_solidtime_config(&ctx.store, &server.uri()).await;

    let handle = ctx.handle();
    let report = pull_now(handle.clone(), handle.state()).await.unwrap();
    assert_eq!(report.inserted, 0);
    assert_eq!(report.updated, 0);
    assert_eq!(report.deleted, 0);
    assert!(report.adopted.is_none());
    assert!(report.conflict.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn pull_now_inserts_remote_entry_into_local_store() {
    let ctx = common::make_app().await;
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {
                    "id": "remote-1",
                    "description": "from web",
                    "project_id": null,
                    "task_id": null,
                    "start": "2026-05-20T09:00:00Z",
                    "end":   "2026-05-20T10:00:00Z",
                    "billable": false,
                    "updated_at": "2026-05-20T10:00:01Z"
                }
            ]
        })))
        .mount(&server)
        .await;

    seed_solidtime_config(&ctx.store, &server.uri()).await;

    let handle = ctx.handle();
    let report = pull_now(handle.clone(), handle.state()).await.unwrap();
    assert_eq!(report.inserted, 1);

    let entries = Entries::new((*ctx.store).clone())
        .get_by_solidtime_id("remote-1")
        .await
        .unwrap()
        .expect("remote entry adopted locally");
    assert_eq!(entries.description, "from web");
    assert_eq!(entries.sync_state, "synced");
    assert_eq!(entries.source, "solidtime");
}

#[tokio::test(flavor = "multi_thread")]
async fn conflict_resolve_dismiss_makes_no_remote_calls() {
    let ctx = common::make_app().await;
    let server = MockServer::start().await;
    // No mocks mounted on purpose: if dismiss accidentally calls the
    // network, wiremock will record an unmatched request and the test
    // can inspect it via received_requests().
    seed_solidtime_config(&ctx.store, &server.uri()).await;

    let handle = ctx.handle();
    conflict_resolve(
        handle.clone(),
        handle.state(),
        ConflictResolveArgs {
            action: ConflictActionDto::Dismiss,
            remote_id: "remote-1".into(),
        },
    )
    .await
    .unwrap();

    let received = server.received_requests().await.unwrap_or_default();
    assert!(
        received.is_empty(),
        "dismiss should not make HTTP calls; got {} request(s)",
        received.len()
    );
}
