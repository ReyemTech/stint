mod common;

use stint_core::store::entries::Entries;
use stint_core::store::queue::Queue;
use stint_core::solidtime::SolidtimeClient;
use stint_core::sync::push::push_one;
use stint_core::timer::{StartArgs, TimerService};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn push_one_succeeds_for_create_entry_and_marks_synced() {
    let env = common::setup().await;
    let timer = TimerService::new(env.store.clone());
    let id = timer.start(StartArgs {
        description: "do thing".into(), project_id: None, task_id: None, source: "cli".into(),
    }).await.unwrap();

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": { "id": "remote-1", "description": "do thing", "start": "2026-05-17T09:00:00Z" }
        })))
        .mount(&server).await;

    let client = SolidtimeClient::new(&server.uri(), "t").with_org("org-1");
    let queue = Queue::new(env.store.clone());

    let due = queue.take_due(10).await.unwrap();
    assert_eq!(due.len(), 1);

    push_one(&env.store, &client, &due[0]).await.unwrap();

    // Queue is empty
    assert!(queue.take_due(10).await.unwrap().is_empty());
    // Entry is now synced with solidtime_id
    let row = Entries::new(env.store.clone()).get(&id).await.unwrap().unwrap();
    assert_eq!(row.sync_state, "synced");
    assert_eq!(row.solidtime_id.as_deref(), Some("remote-1"));
}
