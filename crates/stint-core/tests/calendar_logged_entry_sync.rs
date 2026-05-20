mod common;

use stint_core::solidtime::SolidtimeClient;
use stint_core::store::entries::{Entries, NewCompletedEntry};
use stint_core::store::queue::{Queue, QueueOp};
use stint_core::sync::drain_once;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn logged_calendar_entry_drains_through_sync_queue() {
    let env = common::setup().await;
    let server = MockServer::start().await;

    let settings = stint_core::config::Settings::new(env.store.clone());
    settings.set("solidtime.url", &server.uri()).await.unwrap();
    settings.set("solidtime.org", "org-1").await.unwrap();
    settings.set("solidtime.member_id", "mem-1").await.unwrap();

    Mock::given(method("POST"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .and(header("Authorization", "Bearer tok"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": { "id": "remote-1", "description": "Sprint review",
                       "start": "2026-05-19T14:00:00Z", "end": "2026-05-19T15:00:00Z" }
        })))
        .mount(&server)
        .await;

    let entries = Entries::new(env.store.clone());
    let local_uuid = entries
        .create_completed(NewCompletedEntry {
            description: "Sprint review".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-19T14:00:00Z".into(),
            end_at: "2026-05-19T15:00:00Z".into(),
            billable: false,
            source: "calendar".into(),
            source_event_id: Some("acc-1:evt-1:2026-05-19T14:00:00Z".into()),
        })
        .await
        .unwrap();

    // create_completed does not auto-enqueue (mirrors the timer/stop flow where
    // enqueueing happens in the caller). Enqueue a create_entry op manually,
    // matching the pattern used by other sync_push tests.
    let queue = Queue::new(env.store.clone());
    queue
        .enqueue(
            QueueOp::CreateEntry,
            &serde_json::json!({ "local_uuid": local_uuid }).to_string(),
            Some(&local_uuid),
        )
        .await
        .unwrap();

    let client = SolidtimeClient::with_api_token(&server.uri(), "tok").with_org("org-1");
    let drained = drain_once(&env.store, &client).await.unwrap();
    assert_eq!(drained, 1);

    let row = entries.get(&local_uuid).await.unwrap().unwrap();
    assert_eq!(row.sync_state, "synced");
    assert_eq!(row.solidtime_id.as_deref(), Some("remote-1"));
}
