use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use std::sync::Arc;
use stint_core::calendar::google::{build_provider_from_blob, resolve_account_identifier};
use stint_core::calendar::google::client::GoogleClient;
use stint_core::calendar::provider::RemoteCalendar;
use stint_core::calendar::store::{calendar_blob_delete, calendar_blob_save, CalendarOAuthBlob};
use stint_core::calendar::google::GoogleProvider;
use stint_core::calendar::provider::CalendarProvider;
use stint_core::calendar::types::{ProviderKind, TimeRange};
use stint_core::config::secrets::Secrets;
use stint_core::oauth::tokens::TokenSet;
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

struct BlobCleanup {
    secrets: Secrets,
    account_id: String,
}

impl BlobCleanup {
    fn new(secrets: &Secrets, account_id: &str) -> Self {
        Self {
            secrets: secrets.clone(),
            account_id: account_id.to_string(),
        }
    }
}

impl Drop for BlobCleanup {
    fn drop(&mut self) {
        let _ = calendar_blob_delete(&self.secrets, &self.account_id);
    }
}

fn unique_prefix() -> String {
    format!("tech.reyem.stint-test.{}", uuid::Uuid::new_v4().simple())
}

fn test_blob(client_secret: Option<&str>) -> CalendarOAuthBlob {
    CalendarOAuthBlob {
        client_id: "google-client-id".into(),
        client_secret: client_secret.map(str::to_string),
        tokens: TokenSet::from_response(
            "access-1".into(),
            Some("refresh-1".into()),
            3600,
            Some("https://www.googleapis.com/auth/calendar.readonly".into()),
            Utc::now(),
        ),
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

#[test]
fn build_provider_returns_missing_config_when_blob_is_absent() {
    if std::env::var_os("STINT_SKIP_KEYCHAIN_TESTS").is_some() {
        eprintln!("skipped: STINT_SKIP_KEYCHAIN_TESTS=1");
        return;
    }

    let secrets = Secrets::with_service_prefix(unique_prefix());
    match build_provider_from_blob(&secrets, "missing-account") {
        Err(stint_core::Error::MissingConfig(key)) => assert_eq!(key, "calendar.oauth"),
        Err(other) => panic!("expected MissingConfig(calendar.oauth), got {other:?}"),
        Ok(_) => panic!("expected missing calendar oauth blob to fail"),
    }
}

#[test]
fn build_provider_constructs_google_provider_from_blob_without_client_secret() {
    if std::env::var_os("STINT_SKIP_KEYCHAIN_TESTS").is_some() {
        eprintln!("skipped: STINT_SKIP_KEYCHAIN_TESTS=1");
        return;
    }

    let secrets = Secrets::with_service_prefix(unique_prefix());
    let account_id = "google-no-secret";
    let _cleanup = BlobCleanup::new(&secrets, account_id);
    let blob = test_blob(None);

    calendar_blob_save(&secrets, account_id, &blob).unwrap();

    let provider = build_provider_from_blob(&secrets, account_id).unwrap();
    assert_eq!(provider.kind(), ProviderKind::Google);
}

#[test]
fn build_provider_constructs_google_provider_from_blob_with_client_secret() {
    if std::env::var_os("STINT_SKIP_KEYCHAIN_TESTS").is_some() {
        eprintln!("skipped: STINT_SKIP_KEYCHAIN_TESTS=1");
        return;
    }

    let secrets = Secrets::with_service_prefix(unique_prefix());
    let account_id = "google-with-secret";
    let _cleanup = BlobCleanup::new(&secrets, account_id);
    let blob = test_blob(Some("desktop-client-secret"));

    calendar_blob_save(&secrets, account_id, &blob).unwrap();

    let provider = build_provider_from_blob(&secrets, account_id).unwrap();
    assert_eq!(provider.kind(), ProviderKind::Google);
}

#[test]
fn build_provider_surfaces_corrupt_blob_errors() {
    if std::env::var_os("STINT_SKIP_KEYCHAIN_TESTS").is_some() {
        eprintln!("skipped: STINT_SKIP_KEYCHAIN_TESTS=1");
        return;
    }

    let secrets = Secrets::with_service_prefix(unique_prefix());
    let account_id = "corrupt-account";
    let _cleanup = BlobCleanup::new(&secrets, account_id);
    let key = format!("calendar.{account_id}");
    secrets.set(&key, "not json").unwrap();

    match build_provider_from_blob(&secrets, account_id) {
        Err(stint_core::Error::OAuthServer(msg)) => {
            assert!(msg.starts_with("Calendar Keychain blob malformed for corrupt-account:"));
        }
        Err(other) => panic!("expected OAuthServer(malformed...), got {other:?}"),
        Ok(_) => panic!("expected corrupt blob load to fail"),
    }
}

#[test]
fn resolve_identifier_falls_back_to_id_when_name_is_empty() {
    let cals = vec![RemoteCalendar {
        id: "empty-name@example.com".into(),
        name: String::new(),
        color: None,
        primary: false,
    }];

    assert_eq!(
        resolve_account_identifier(&cals, "fallback-uuid"),
        "empty-name@example.com"
    );
}
