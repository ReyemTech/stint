//! Integration tests for `commands/projects.rs`.

mod common;

use stint_app::commands::projects::{list_organizations, list_projects, refresh_projects};
use stint_core::config::secrets::Secrets;
use stint_core::config::Settings;
use stint_core::store::reference::{ProjectRow, Reference};
use tauri::Manager;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn seed_solidtime_config(
    store: &std::sync::Arc<stint_core::store::Store>,
    url: &str,
    org: Option<&str>,
) {
    let settings = Settings::new((**store).clone());
    settings.set("solidtime.url", url).await.unwrap();
    if let Some(o) = org {
        settings.set("solidtime.org", o).await.unwrap();
    }
    // STINT_SECRET_PREFIX has already been set by common::make_app so this
    // write lands under the synthetic test prefix, not the real Keychain.
    Secrets::default()
        .set("solidtime.token", "test-token")
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn list_projects_returns_empty_on_fresh_store() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();
    let rows = list_projects(handle.state()).await.unwrap();
    assert!(rows.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn list_projects_returns_seeded_rows() {
    let ctx = common::make_app().await;
    Reference::new((*ctx.store).clone())
        .upsert_projects(&[ProjectRow {
            id: "p-1".into(),
            name: "Tet".into(),
            color: None,
            client_id: None,
            archived: 0,
        }])
        .await
        .unwrap();

    let handle = ctx.handle();
    let rows = list_projects(handle.state()).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "p-1");
    assert_eq!(rows[0].name, "Tet");
}

#[tokio::test(flavor = "multi_thread")]
async fn list_organizations_fetches_memberships_from_solidtime() {
    let ctx = common::make_app().await;
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/users/me/memberships"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {
                    "id": "m-1",
                    "organization": { "id": "org-1", "name": "Acme", "currency": "USD" },
                    "role": "admin"
                }
            ]
        })))
        .mount(&server)
        .await;

    seed_solidtime_config(&ctx.store, &server.uri(), None).await;

    let handle = ctx.handle();
    let orgs = list_organizations(handle.state()).await.unwrap();
    assert_eq!(orgs.len(), 1);
    assert_eq!(orgs[0].id, "org-1");
    assert_eq!(orgs[0].member_id, "m-1");
    assert_eq!(orgs[0].name, "Acme");
}

#[tokio::test(flavor = "multi_thread")]
async fn refresh_projects_populates_local_cache_from_solidtime() {
    let ctx = common::make_app().await;
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/projects"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                { "id": "p1", "name": "Tet", "color": null, "client_id": null, "archived": false }
            ]
        })))
        .mount(&server)
        .await;
    for endpoint in ["tasks", "tags"] {
        Mock::given(method("GET"))
            .and(path(format!("/api/v1/organizations/org-1/{endpoint}")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "data": [] })),
            )
            .mount(&server)
            .await;
    }

    seed_solidtime_config(&ctx.store, &server.uri(), Some("org-1")).await;

    let handle = ctx.handle();
    let n = refresh_projects(handle.state()).await.unwrap();
    assert_eq!(n, 1);

    let rows = list_projects(handle.state()).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "p1");
}

#[tokio::test(flavor = "multi_thread")]
async fn refresh_projects_errors_when_solidtime_url_missing() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();
    let err = refresh_projects(handle.state()).await.unwrap_err();
    assert!(err.message.contains("solidtime.url"), "got: {}", err.message);
}
