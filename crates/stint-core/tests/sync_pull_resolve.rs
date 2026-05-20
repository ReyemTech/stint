mod common;

use stint_core::config::Settings;
use stint_core::solidtime::SolidtimeClient;
use stint_core::store::entries::{Entries, NewTimeEntry};
use stint_core::store::queue::Queue;
use stint_core::store::running::RunningTimer;
use stint_core::sync::pull::{resolve_conflict, ConflictAction};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn configure(env: &common::TestEnv, server_uri: &str) {
    let s = Settings::new(env.store.clone());
    s.set("solidtime.url", server_uri).await.unwrap();
    s.set("solidtime.org", "org-1").await.unwrap();
    s.set("solidtime.member_id", "m-1").await.unwrap();
}

#[tokio::test]
async fn dismiss_is_a_noop() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    // Pre-state: nothing.
    assert!(RunningTimer::new(env.store.clone())
        .get()
        .await
        .unwrap()
        .is_none());

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    resolve_conflict(&env.store, &client, ConflictAction::Dismiss, "remote-x")
        .await
        .unwrap();

    // Post-state: still nothing — no rows written, no queue ops.
    assert!(RunningTimer::new(env.store.clone())
        .get()
        .await
        .unwrap()
        .is_none());
    assert!(Queue::new(env.store.clone())
        .take_due(10)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn stop_remote_mirrors_remote_then_marks_dirty_and_enqueues_update() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    // Wiremock the GET that StopRemote performs.
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries/remote-stop"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "id": "remote-stop",
                "description": "remote task",
                "start": "2026-05-20T16:00:00Z",
                "end": null,
                "billable": false,
                "updated_at": "2026-05-20T16:00:00Z"
            }
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    resolve_conflict(
        &env.store,
        &client,
        ConflictAction::StopRemote,
        "remote-stop",
    )
    .await
    .unwrap();

    // Local row created from remote + end_at set + flipped to dirty.
    let entries = Entries::new(env.store.clone());
    let row = entries
        .get_by_solidtime_id("remote-stop")
        .await
        .unwrap()
        .unwrap();
    assert!(row.end_at.is_some(), "end_at should be set");
    assert_eq!(row.sync_state, "dirty");

    // Queue should have one UpdateEntry op for this row.
    let due = Queue::new(env.store.clone()).take_due(10).await.unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].op, "update_entry");
    assert_eq!(due[0].entry_uuid.as_deref(), Some(row.local_uuid.as_str()));
}

#[tokio::test]
async fn stop_remote_errors_when_remote_already_gone() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    Mock::given(method("GET"))
        .and(path(
            "/api/v1/organizations/org-1/time-entries/remote-vanished",
        ))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let err = resolve_conflict(
        &env.store,
        &client,
        ConflictAction::StopRemote,
        "remote-vanished",
    )
    .await
    .unwrap_err();
    assert!(
        matches!(err, stint_core::Error::NotFound(_)),
        "expected NotFound, got {err:?}"
    );
}

#[tokio::test]
async fn switch_stops_local_then_adopts_remote() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    // Set up a local running timer.
    let entries = Entries::new(env.store.clone());
    let local_uuid = entries
        .create(NewTimeEntry {
            description: "local task".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T16:00:00Z".into(),
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();
    RunningTimer::new(env.store.clone())
        .set(&local_uuid)
        .await
        .unwrap();

    // List call inside the inner pull returns the remote running entry.
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "remote-target",
                "description": "remote task",
                "start": "2026-05-20T16:30:00Z",
                "end": null,
                "billable": false,
                "updated_at": "2026-05-20T16:30:00Z"
            }]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    resolve_conflict(&env.store, &client, ConflictAction::Switch, "remote-target")
        .await
        .unwrap();

    // Local timer was stopped (end_at set, dirty).
    let local_row = entries.get(&local_uuid).await.unwrap().unwrap();
    assert!(
        local_row.end_at.is_some(),
        "local timer should have been stopped"
    );

    // Remote was adopted.
    let adopted = entries
        .get_by_solidtime_id("remote-target")
        .await
        .unwrap()
        .unwrap();
    let running = RunningTimer::new(env.store.clone())
        .get()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(running.local_uuid, adopted.local_uuid);
}
