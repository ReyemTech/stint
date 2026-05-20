use chrono::{TimeZone, Utc};
use stint_core::calendar::google::client::GoogleClient;
use stint_core::calendar::types::TimeRange;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn range_today() -> TimeRange {
    TimeRange {
        start: Utc.with_ymd_and_hms(2026, 5, 19, 0, 0, 0).unwrap(),
        end: Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
    }
}

#[tokio::test]
async fn list_calendars_calls_calendar_list_with_bearer() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/users/me/calendarList"))
        .and(header("Authorization", "Bearer access-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [
                { "id": "me@example.com", "summary": "me@example.com", "backgroundColor": "#abc", "primary": true },
                { "id": "work@example.com", "summary": "Work" }
            ]
        })))
        .mount(&server)
        .await;

    let client = GoogleClient::with_base_url(&server.uri());
    let cals = client.list_calendars("access-1").await.unwrap();
    assert_eq!(cals.len(), 2);
    assert_eq!(cals[0].id, "me@example.com");
    assert_eq!(cals[0].name, "me@example.com");
    assert_eq!(cals[0].color.as_deref(), Some("#abc"));
    assert!(cals[0].primary, "first item should be primary");
    assert_eq!(cals[1].id, "work@example.com");
    assert!(!cals[1].primary, "second item should not be primary");
}

#[tokio::test]
async fn list_events_calls_events_with_range_and_single_events() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/calendars/primary/events"))
        .and(header("Authorization", "Bearer access-1"))
        .and(query_param("singleEvents", "true"))
        .and(query_param("orderBy", "startTime"))
        .and(query_param("timeMin", "2026-05-19T00:00:00+00:00"))
        .and(query_param("timeMax", "2026-05-20T00:00:00+00:00"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [
                {
                    "id": "evt-1",
                    "summary": "Standup",
                    "start": { "dateTime": "2026-05-19T09:00:00Z" },
                    "end":   { "dateTime": "2026-05-19T09:15:00Z" },
                    "attendees": [
                        { "self": true, "responseStatus": "accepted" },
                        { "self": false, "responseStatus": "declined" }
                    ]
                },
                {
                    "id": "evt-2",
                    "summary": "All-hands",
                    "start": { "date": "2026-05-19" },
                    "end":   { "date": "2026-05-20" }
                },
                {
                    "id": "evt-3",
                    "summary": "Recurring 1:1",
                    "start": { "dateTime": "2026-05-19T11:00:00Z" },
                    "end":   { "dateTime": "2026-05-19T11:30:00Z" },
                    "recurringEventId": "evt-root"
                }
            ]
        })))
        .mount(&server)
        .await;

    let client = GoogleClient::with_base_url(&server.uri());
    let evs = client
        .list_events("access-1", "primary", range_today())
        .await
        .unwrap();
    assert_eq!(evs.len(), 3);

    assert_eq!(evs[0].id, "evt-1");
    assert_eq!(evs[0].title, "Standup");
    assert_eq!(evs[0].start_at, "2026-05-19T09:00:00Z");
    assert!(!evs[0].is_all_day);
    assert_eq!(
        evs[0].attendee_status,
        Some(stint_core::calendar::types::AttendeeStatus::Accepted)
    );

    assert_eq!(evs[1].title, "All-hands");
    assert!(evs[1].is_all_day);
    assert_eq!(evs[1].start_at, "2026-05-19");

    assert_eq!(evs[2].recurring_root.as_deref(), Some("evt-root"));
}

#[tokio::test]
async fn list_events_maps_401_to_auth_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/calendars/primary/events"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let client = GoogleClient::with_base_url(&server.uri());
    let err = client
        .list_events("access-1", "primary", range_today())
        .await
        .unwrap_err();
    assert!(matches!(err, stint_core::Error::OAuthRefreshFailed));
}

#[tokio::test]
async fn list_events_paginates_with_next_page_token() {
    let server = MockServer::start().await;
    // Page 1.
    Mock::given(method("GET"))
        .and(path("/calendar/v3/calendars/primary/events"))
        .and(query_param("singleEvents", "true"))
        .and(query_param("orderBy", "startTime"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [
                { "id": "evt-1", "summary": "First",
                  "start": { "dateTime": "2026-05-19T09:00:00Z" },
                  "end":   { "dateTime": "2026-05-19T09:15:00Z" } }
            ],
            "nextPageToken": "tok-2"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    // Page 2.
    Mock::given(method("GET"))
        .and(path("/calendar/v3/calendars/primary/events"))
        .and(query_param("pageToken", "tok-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [
                { "id": "evt-2", "summary": "Second",
                  "start": { "dateTime": "2026-05-19T10:00:00Z" },
                  "end":   { "dateTime": "2026-05-19T10:30:00Z" } }
            ]
        })))
        .mount(&server)
        .await;

    let client = GoogleClient::with_base_url(&server.uri());
    let evs = client
        .list_events("access-1", "primary", range_today())
        .await
        .unwrap();
    assert_eq!(evs.len(), 2);
    assert_eq!(evs[0].id, "evt-1");
    assert_eq!(evs[1].id, "evt-2");
}
