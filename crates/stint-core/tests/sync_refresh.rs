mod common;

use stint_core::solidtime::SolidtimeClient;
use stint_core::store::reference::Reference;
use stint_core::sync::refresh::refresh_reference_data;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn refresh_reference_data_writes_clients_projects_tasks_tags() {
    let env = common::setup().await;
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/clients"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{ "id": "c1", "name": "Acme", "archived": false }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/projects"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{ "id": "p1", "name": "Tet", "color": null, "client_id": "c1", "archived": false }]
        })))
        .mount(&server).await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/tasks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{ "id": "t1", "project_id": "p1", "name": "T", "done": false }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{ "id": "g1", "name": "billable" }]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    refresh_reference_data(&env.store, &client).await.unwrap();

    let r = Reference::new(env.store.clone());
    let clients = r.list_clients().await.unwrap();
    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0].name, "Acme");
    assert_eq!(r.list_projects().await.unwrap().len(), 1);
    assert_eq!(r.list_tasks("p1").await.unwrap().len(), 1);
    assert_eq!(r.list_tags().await.unwrap().len(), 1);
}
