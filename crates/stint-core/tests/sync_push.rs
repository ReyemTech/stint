mod common;

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
    let timer = TimerService::new(env.store.clone());
    let id = timer
        .start(StartArgs {
            description: "do thing".into(),
            project_id: None,
            task_id: None,
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

    let client = SolidtimeClient::new(&server.uri(), "t").with_org("org-1");
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

    let client = SolidtimeClient::new(&server.uri(), "t").with_org("org-1");
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

    let client = SolidtimeClient::new(&server.uri(), "t").with_org("org-1");
    push_one(&env.store, &client, &delete_row).await.unwrap();
}
