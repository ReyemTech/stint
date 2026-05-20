use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use std::sync::Arc;
use stint_core::calendar::google::client::GoogleClient;
use stint_core::calendar::google::GoogleProvider;
use stint_core::calendar::provider::CalendarProvider;
use stint_core::calendar::types::{ProviderKind, TimeRange};
use stint_core::solidtime::auth::TokenProvider;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

struct FixedToken(String);

#[async_trait]
impl TokenProvider for FixedToken {
    async fn access_token(&self) -> stint_core::Result<String> {
        Ok(self.0.clone())
    }
}

#[tokio::test]
async fn provider_kind_is_google() {
    let server = MockServer::start().await;
    let p = GoogleProvider::new(
        Arc::new(FixedToken("t".into())),
        GoogleClient::with_base_url(&server.uri()),
    );
    assert_eq!(p.kind(), ProviderKind::Google);
}

#[tokio::test]
async fn list_calendars_passes_token_to_client() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/users/me/calendarList"))
        .and(header("Authorization", "Bearer the-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [{ "id": "primary", "summary": "Primary" }]
        })))
        .mount(&server)
        .await;
    let p = GoogleProvider::new(
        Arc::new(FixedToken("the-token".into())),
        GoogleClient::with_base_url(&server.uri()),
    );
    let cals = p.list_calendars().await.unwrap();
    assert_eq!(cals.len(), 1);
}

#[tokio::test]
async fn list_events_proxies_to_client() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/calendars/primary/events"))
        .and(header("Authorization", "Bearer the-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [
                { "id": "evt-1", "summary": "Standup",
                  "start": { "dateTime": "2026-05-19T09:00:00Z" },
                  "end":   { "dateTime": "2026-05-19T09:15:00Z" } }
            ]
        })))
        .mount(&server)
        .await;
    let p = GoogleProvider::new(
        Arc::new(FixedToken("the-token".into())),
        GoogleClient::with_base_url(&server.uri()),
    );
    let range = TimeRange {
        start: Utc.with_ymd_and_hms(2026, 5, 19, 0, 0, 0).unwrap(),
        end: Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
    };
    let evs = p.list_events("primary", range).await.unwrap();
    assert_eq!(evs.len(), 1);
    assert_eq!(evs[0].title, "Standup");
}
