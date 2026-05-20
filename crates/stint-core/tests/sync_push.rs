mod common;

use stint_core::config::Settings;
use stint_core::solidtime::SolidtimeClient;
use stint_core::store::entries::Entries;
use stint_core::store::queue::Queue;
use stint_core::sync::push::push_one;
use stint_core::timer::{StartArgs, TimerService};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn push_one_succeeds_for_create_entry_and_marks_synced() {
    let env = common::setup().await;
    Settings::new(env.store.clone())
        .set("solidtime.member_id", "m-1")
        .await
        .unwrap();
    let timer = TimerService::new(env.store.clone());
    let id = timer
        .start(StartArgs {
            description: "do thing".into(),
            project_id: None,
            task_id: None,
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": { "id": "remote-1", "description": "do thing", "start": "2026-05-17T09:00:00Z" }
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let queue = Queue::new(env.store.clone());

    let due = queue.take_due(10).await.unwrap();
    assert_eq!(due.len(), 1);

    push_one(&env.store, &client, &due[0]).await.unwrap();

    // Queue is empty
    assert!(queue.take_due(10).await.unwrap().is_empty());
    // Entry is now synced with solidtime_id
    let row = Entries::new(env.store.clone())
        .get(&id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.sync_state, "synced");
    assert_eq!(row.solidtime_id.as_deref(), Some("remote-1"));
}

#[tokio::test]
async fn push_one_marks_failed_on_500_and_backs_off() {
    let env = common::setup().await;
    let timer = TimerService::new(env.store.clone());
    timer
        .start(StartArgs {
            description: "x".into(),
            project_id: None,
            task_id: None,
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let queue = Queue::new(env.store.clone());
    let due = queue.take_due(10).await.unwrap();

    let result = push_one(&env.store, &client, &due[0]).await;
    assert!(result.is_err());

    // Item should still be in queue but not due (backed off).
    assert!(queue.take_due(10).await.unwrap().is_empty());
}

#[tokio::test]
async fn push_one_handles_delete_entry() {
    let env = common::setup().await;

    // Create + sync + delete to enqueue a delete op
    let timer = TimerService::new(env.store.clone());
    let id = timer
        .start(StartArgs {
            description: "x".into(),
            project_id: None,
            task_id: None,
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();
    let entries = Entries::new(env.store.clone());
    entries.mark_synced(&id, "remote-1").await.unwrap();

    // Manually enqueue delete (the proper helper comes in Task 23)
    let queue = Queue::new(env.store.clone());
    queue
        .enqueue(
            stint_core::store::queue::QueueOp::DeleteEntry,
            &serde_json::json!({ "local_uuid": id, "solidtime_id": "remote-1" }).to_string(),
            Some(&id),
        )
        .await
        .unwrap();
    // Drop the older create_entry op so the delete is the only one
    let due = queue.take_due(10).await.unwrap();
    let delete_row = due.iter().find(|r| r.op == "delete_entry").unwrap().clone();
    for r in due.iter().filter(|r| r.op != "delete_entry") {
        queue.mark_succeeded(r.id).await.unwrap();
    }

    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/api/v1/organizations/org-1/time-entries/remote-1"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    push_one(&env.store, &client, &delete_row).await.unwrap();
}

#[tokio::test]
async fn push_update_handles_remote_404_by_deleting_local_and_succeeding() {
    let env = common::setup().await;
    Settings::new(env.store.clone())
        .set("solidtime.member_id", "m-1")
        .await
        .unwrap();

    // Seed a synced local row that has a pending update queued.
    let entries = Entries::new(env.store.clone());
    let local_uuid = entries
        .create(stint_core::store::entries::NewTimeEntry {
            description: "test 2".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T16:00:00Z".into(),
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();
    entries
        .mark_synced(&local_uuid, "remote-gone")
        .await
        .unwrap();
    entries
        .update_description(&local_uuid, "test 2 edited")
        .await
        .unwrap();

    let queue = Queue::new(env.store.clone());
    queue
        .enqueue(
            stint_core::store::queue::QueueOp::UpdateEntry,
            &serde_json::json!({ "local_uuid": local_uuid }).to_string(),
            Some(&local_uuid),
        )
        .await
        .unwrap();

    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/api/v1/organizations/org-1/time-entries/remote-gone"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let due = queue.take_due(10).await.unwrap();
    assert_eq!(due.len(), 1);

    push_one(&env.store, &client, &due[0]).await.unwrap();

    // Queue op succeeded — row is gone.
    assert!(queue.take_due(10).await.unwrap().is_empty());
    // Local row is hard-deleted.
    assert!(entries.get(&local_uuid).await.unwrap().is_none());
}

#[tokio::test]
async fn push_update_404_clears_running_timer_when_running_row_disappears() {
    let env = common::setup().await;
    Settings::new(env.store.clone())
        .set("solidtime.member_id", "m-1")
        .await
        .unwrap();

    // Build the running-row state directly (skip TimerService::start so we
    // don't have to drain the create_entry op it enqueues).
    let entries = Entries::new(env.store.clone());
    let local_uuid = entries
        .create(stint_core::store::entries::NewTimeEntry {
            description: "running task".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T16:00:00Z".into(),
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();
    entries
        .mark_synced(&local_uuid, "remote-gone")
        .await
        .unwrap();
    stint_core::store::running::RunningTimer::new(env.store.clone())
        .set(&local_uuid)
        .await
        .unwrap();
    // Edit flips synced → dirty.
    entries
        .update_description(&local_uuid, "renamed")
        .await
        .unwrap();
    let queue = Queue::new(env.store.clone());
    queue
        .enqueue(
            stint_core::store::queue::QueueOp::UpdateEntry,
            &serde_json::json!({ "local_uuid": local_uuid }).to_string(),
            Some(&local_uuid),
        )
        .await
        .unwrap();

    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/api/v1/organizations/org-1/time-entries/remote-gone"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let due = queue.take_due(10).await.unwrap();
    push_one(&env.store, &client, &due[0]).await.unwrap();

    // Local row gone, running_timer cleared.
    assert!(entries.get(&local_uuid).await.unwrap().is_none());
    assert!(
        stint_core::store::running::RunningTimer::new(env.store.clone())
            .get()
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn delete_time_entry_treats_404_as_success() {
    // The solidtime client should NOT error on DELETE 404 — the entry is
    // already gone, which is what the caller wanted.
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path(
            "/api/v1/organizations/org-1/time-entries/already-gone",
        ))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    client
        .delete_time_entry("already-gone")
        .await
        .expect("404 on DELETE should be treated as success");
}
