mod common;

use stint_core::solidtime::SolidtimeClient;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn list_time_entries_sends_member_id_and_window_params() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .and(query_param("member_ids[]", "m-1"))
        .and(query_param("start", "2026-05-19T17:00:00Z"))
        .and(query_param("end", "2026-05-20T17:00:00Z"))
        .and(header("authorization", "Bearer t"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "remote-1",
                "description": "in progress",
                "start": "2026-05-20T16:30:00Z",
                "end": null,
                "billable": false,
                "updated_at": "2026-05-20T16:30:00Z"
            }]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let entries = client
        .list_time_entries("m-1", "2026-05-19T17:00:00Z", "2026-05-20T17:00:00Z")
        .await
        .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "remote-1");
    assert!(entries[0].end.is_none());
}

#[tokio::test]
async fn list_time_entries_unauth_maps_to_solidtime_auth_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let err = client
        .list_time_entries("m-1", "a", "b")
        .await
        .unwrap_err();
    assert!(matches!(err, stint_core::Error::SolidtimeAuth), "got: {err:?}");
}

#[tokio::test]
async fn get_time_entry_returns_some_on_200() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries/remote-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "id": "remote-1",
                "description": "still here",
                "start": "2026-05-20T10:00:00Z",
                "end": "2026-05-20T11:00:00Z",
                "billable": true
            }
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let entry = client.get_time_entry("remote-1").await.unwrap();
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().id, "remote-1");
}

#[tokio::test]
async fn get_time_entry_returns_none_on_404() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries/gone"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let entry = client.get_time_entry("gone").await.unwrap();
    assert!(entry.is_none());
}
