mod common;

use stint_core::config::Settings;
use stint_core::solidtime::SolidtimeClient;
use stint_core::store::entries::Entries;
use stint_core::store::entries::NewTimeEntry;
use stint_core::store::running::RunningTimer;
use stint_core::sync::pull::{pull, Trigger};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn configure(env: &common::TestEnv, server_uri: &str) {
    let s = Settings::new(env.store.clone());
    s.set("solidtime.url", server_uri).await.unwrap();
    s.set("solidtime.org", "org-1").await.unwrap();
    s.set("solidtime.member_id", "m-1").await.unwrap();
}

#[tokio::test]
async fn adopts_remote_running_when_local_idle() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "remote-running",
                "description": "started in web",
                "start": "2026-05-20T16:30:00Z",
                "end": null,
                "billable": false,
                "updated_at": "2026-05-20T16:30:00Z"
            }]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");

    // Precondition: no running timer locally.
    let running = RunningTimer::new(env.store.clone());
    assert!(running.get().await.unwrap().is_none());

    let report = pull(&env.store, &client, Trigger::OnStartup).await.unwrap();

    // Adoption succeeded.
    assert!(report.adopted.is_some());
    assert!(report.conflict.is_none());
    let adopted_uuid = report.adopted.unwrap();

    let entries = Entries::new(env.store.clone());
    let row = entries.get(&adopted_uuid).await.unwrap().unwrap();
    assert_eq!(row.solidtime_id.as_deref(), Some("remote-running"));
    assert_eq!(row.source, "solidtime");
    assert_eq!(row.sync_state, "synced");
    assert!(row.end_at.is_none());

    let running_row = running.get().await.unwrap().unwrap();
    assert_eq!(running_row.local_uuid, adopted_uuid);
}

#[tokio::test]
async fn does_nothing_when_remote_idle_and_local_idle() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": []})))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::OnStartup).await.unwrap();
    assert!(report.adopted.is_none());
    assert!(report.conflict.is_none());
}

#[tokio::test]
async fn does_nothing_when_remote_idle_but_local_running() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    let entries = Entries::new(env.store.clone());
    let local_uuid = entries
        .create(NewTimeEntry {
            description: "local-only".into(),
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

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": []})))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::OnStartup).await.unwrap();
    assert!(report.adopted.is_none());
    assert!(report.conflict.is_none());

    // Local timer remained intact.
    let running = RunningTimer::new(env.store.clone())
        .get()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(running.local_uuid, local_uuid);
}

#[tokio::test]
async fn no_op_when_remote_and_local_share_same_id() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    // Local timer is already the same entry (e.g. adopted on a previous pull).
    let entries = Entries::new(env.store.clone());
    let local_uuid = entries
        .create_from_remote(stint_core::store::entries::RemoteEntryUpsert {
            solidtime_id: "remote-same".into(),
            description: "started in web".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T16:30:00Z".into(),
            end_at: None,
            billable: false,
            updated_at: "2026-05-20T16:30:00Z".into(),
        })
        .await
        .unwrap();
    RunningTimer::new(env.store.clone())
        .set(&local_uuid)
        .await
        .unwrap();

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "remote-same",
                "description": "started in web",
                "start": "2026-05-20T16:30:00Z",
                "end": null,
                "billable": false,
                "updated_at": "2026-05-20T16:30:00Z"
            }]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::OnStartup).await.unwrap();
    assert!(report.adopted.is_none(), "no new adoption");
    assert!(report.conflict.is_none(), "no conflict");
}

#[tokio::test]
async fn surfaces_conflict_when_local_and_remote_differ() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

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

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "remote-other",
                "description": "other device",
                "start": "2026-05-20T16:30:00Z",
                "end": null,
                "billable": false,
                "updated_at": "2026-05-20T16:30:00Z"
            }]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::OnStartup).await.unwrap();
    assert!(
        report.adopted.is_none(),
        "must not silently overwrite local"
    );
    let conflict = report.conflict.expect("conflict should be surfaced");
    assert_eq!(conflict.remote_id, "remote-other");
    assert_eq!(conflict.local_local_uuid, local_uuid);
    assert_eq!(conflict.local_description, "local task");

    // Local timer still ticking; no new entry inserted.
    let running = RunningTimer::new(env.store.clone())
        .get()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(running.local_uuid, local_uuid);
}

#[tokio::test]
async fn ignores_completed_remote_entries_when_picking_running() {
    // Verifies the `find(|e| e.end.is_none())` filter — a completed entry
    // (end set) must not be mistaken for the running one.
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "remote-finished",
                "description": "done in web",
                "start": "2026-05-20T10:00:00Z",
                "end": "2026-05-20T11:00:00Z",
                "billable": false,
                "updated_at": "2026-05-20T11:00:00Z"
            }]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::OnStartup).await.unwrap();
    // No running timer to adopt — only a completed entry was returned.
    assert!(report.adopted.is_none());
    assert!(report.conflict.is_none());
    assert!(RunningTimer::new(env.store.clone())
        .get()
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn clears_running_when_local_timer_was_stopped_remotely() {
    // Scenario: user started a timer in stint, it pushed to Solidtime, then
    // the user stopped that same timer from the Solidtime web UI. The pull
    // pipeline must (a) update the local row's end_at via reconcile_history
    // AND (b) clear the running_timer pointer so the UI stops showing it.
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    // Local synced row mirroring a remote that's now been completed.
    let entries = Entries::new(env.store.clone());
    let local_uuid = entries
        .create(NewTimeEntry {
            description: "my task".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T16:00:00Z".into(),
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();
    entries
        .mark_synced(&local_uuid, "remote-stopped")
        .await
        .unwrap();
    RunningTimer::new(env.store.clone())
        .set(&local_uuid)
        .await
        .unwrap();

    // Solidtime returns the same entry — now with end_at set (user stopped
    // it remotely). updated_at is newer than the local's so reconcile_history
    // will apply the update.
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "remote-stopped",
                "description": "my task",
                "start": "2026-05-20T16:00:00Z",
                "end": "2026-05-20T16:30:00Z",
                "billable": false,
                "updated_at": "2030-01-01T00:00:00Z"
            }]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    pull(&env.store, &client, Trigger::Manual).await.unwrap();

    // The local row's end_at was updated by reconcile_history.
    let row = entries.get(&local_uuid).await.unwrap().unwrap();
    assert_eq!(row.end_at.as_deref(), Some("2026-05-20T16:30:00Z"));

    // running_timer was cleared.
    assert!(RunningTimer::new(env.store.clone())
        .get()
        .await
        .unwrap()
        .is_none());
}
