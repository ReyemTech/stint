mod common;

use stint_core::solidtime::SolidtimeClient;
use stint_core::store::reference::{ClientRow, ProjectRow, Reference, TagRow, TaskRow};
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

/// When a project/client/task/tag is deleted on Solidtime, the next
/// refresh should reconcile: projects + clients soft-archived (so
/// historical entries still resolve names), tasks + tags hard-deleted.
#[tokio::test]
async fn refresh_reconciles_remote_side_deletions() {
    let env = common::setup().await;
    let server = MockServer::start().await;

    // Seed local state with TWO of each entity.
    let r = Reference::new(env.store.clone());
    r.upsert_clients(&[
        ClientRow {
            id: "c1".into(),
            name: "Keep".into(),
            archived: 0,
        },
        ClientRow {
            id: "c2".into(),
            name: "ToArchive".into(),
            archived: 0,
        },
    ])
    .await
    .unwrap();
    r.upsert_projects(&[
        ProjectRow {
            id: "p1".into(),
            name: "Keep".into(),
            color: None,
            client_id: None,
            client_name: None,
            archived: 0,
            billable_default: 0,
        },
        ProjectRow {
            id: "p2".into(),
            name: "DeletedRemotely".into(),
            color: None,
            client_id: None,
            client_name: None,
            archived: 0,
            billable_default: 0,
        },
    ])
    .await
    .unwrap();
    r.upsert_tasks(&[
        TaskRow {
            id: "t1".into(),
            project_id: "p1".into(),
            name: "Keep".into(),
            done: 0,
        },
        TaskRow {
            id: "t2".into(),
            project_id: "p1".into(),
            name: "Gone".into(),
            done: 0,
        },
    ])
    .await
    .unwrap();
    r.upsert_tags(&[
        TagRow {
            id: "g1".into(),
            name: "keep".into(),
        },
        TagRow {
            id: "g2".into(),
            name: "gone".into(),
        },
    ])
    .await
    .unwrap();

    // Remote now returns ONLY the c1/p1/t1/g1 rows — c2/p2/t2/g2 disappeared.
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/clients"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{ "id": "c1", "name": "Keep", "archived": false }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/projects"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{ "id": "p1", "name": "Keep", "color": null, "client_id": null, "archived": false }]
        })))
        .mount(&server).await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/tasks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{ "id": "t1", "project_id": "p1", "name": "Keep", "done": false }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{ "id": "g1", "name": "keep" }]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    refresh_reference_data(&env.store, &client).await.unwrap();

    // Projects + clients soft-archived: row still exists, archived = 1.
    let projects = r.list_projects().await.unwrap();
    assert_eq!(projects.len(), 2);
    let p1 = projects.iter().find(|p| p.id == "p1").unwrap();
    let p2 = projects.iter().find(|p| p.id == "p2").unwrap();
    assert_eq!(
        p1.archived, 0,
        "p1 remained on remote, must stay un-archived"
    );
    assert_eq!(p2.archived, 1, "p2 disappeared remotely, must be archived");

    let clients = r.list_clients().await.unwrap();
    let c1 = clients.iter().find(|c| c.id == "c1").unwrap();
    let c2 = clients.iter().find(|c| c.id == "c2").unwrap();
    assert_eq!(c1.archived, 0);
    assert_eq!(c2.archived, 1);

    // Tasks + tags hard-deleted: row is gone entirely.
    let tasks = r.list_tasks("p1").await.unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id, "t1");

    let tags = r.list_tags().await.unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].id, "g1");
}

/// Edge case: if a row was archived locally but is back active on the
/// server, the upsert should un-archive it (covered by the existing
/// upsert path) — and the prune must not re-archive it. Belt + braces.
#[tokio::test]
async fn refresh_does_not_re_archive_a_resurrected_project() {
    let env = common::setup().await;
    let server = MockServer::start().await;

    let r = Reference::new(env.store.clone());
    r.upsert_projects(&[ProjectRow {
        id: "p1".into(),
        name: "WasArchived".into(),
        color: None,
        client_id: None,
        client_name: None,
        archived: 1, // locally archived
        billable_default: 0,
    }])
    .await
    .unwrap();

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/clients"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": []})))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/projects"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{ "id": "p1", "name": "WasArchived", "color": null, "client_id": null, "archived": false }]
        })))
        .mount(&server).await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/tasks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": []})))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": []})))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    refresh_reference_data(&env.store, &client).await.unwrap();

    let p1 = r
        .list_projects()
        .await
        .unwrap()
        .into_iter()
        .find(|p| p.id == "p1")
        .unwrap();
    assert_eq!(p1.archived, 0, "remote says active → local must un-archive");
}
