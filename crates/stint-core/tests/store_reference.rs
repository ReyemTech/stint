mod common;

use stint_core::store::reference::{ClientRow, ProjectRow, Reference, TagRow, TaskRow};

#[tokio::test]
async fn upsert_projects_replaces_set() {
    let env = common::setup().await;
    let r = Reference::new(env.store.clone());

    let initial = vec![
        ProjectRow {
            id: "p1".into(),
            name: "Tet".into(),
            color: None,
            client_id: None,
            client_name: None,
            archived: 0,
            billable_default: 0,
        },
        ProjectRow {
            id: "p2".into(),
            name: "Reyem".into(),
            color: None,
            client_id: None,
            client_name: None,
            archived: 0,
            billable_default: 0,
        },
    ];
    r.upsert_projects(&initial).await.unwrap();

    let list = r.list_projects().await.unwrap();
    assert_eq!(list.len(), 2);

    let updated = vec![ProjectRow {
        id: "p1".into(),
        name: "Tet (renamed)".into(),
        color: Some("#aabbcc".into()),
        client_id: None,
        client_name: None,
        archived: 0,
        billable_default: 0,
    }];
    r.upsert_projects(&updated).await.unwrap();

    let list = r.list_projects().await.unwrap();
    let p1 = list.iter().find(|p| p.id == "p1").unwrap();
    assert_eq!(p1.name, "Tet (renamed)");
}

#[tokio::test]
async fn upsert_tasks_and_tags() {
    let env = common::setup().await;
    let r = Reference::new(env.store.clone());

    r.upsert_projects(&[ProjectRow {
        id: "p1".into(),
        name: "P".into(),
        color: None,
        client_id: None,
        client_name: None,
        archived: 0,
        billable_default: 0,
    }])
    .await
    .unwrap();

    r.upsert_tasks(&[TaskRow {
        id: "t1".into(),
        project_id: "p1".into(),
        name: "Task1".into(),
        done: 0,
    }])
    .await
    .unwrap();

    r.upsert_tags(&[TagRow {
        id: "tag1".into(),
        name: "billable".into(),
    }])
    .await
    .unwrap();

    assert_eq!(r.list_tasks("p1").await.unwrap().len(), 1);
    assert_eq!(r.list_tags().await.unwrap().len(), 1);
}

#[tokio::test]
async fn list_projects_joins_client_name() {
    let env = common::setup().await;
    let r = Reference::new(env.store.clone());

    r.upsert_clients(&[ClientRow {
        id: "c-1".into(),
        name: "Acme".into(),
        archived: 0,
    }])
    .await
    .unwrap();
    r.upsert_projects(&[
        ProjectRow {
            id: "p-1".into(),
            name: "Site".into(),
            color: None,
            client_id: Some("c-1".into()),
            client_name: None,
            archived: 0,
            billable_default: 0,
        },
        ProjectRow {
            id: "p-2".into(),
            name: "Internal".into(),
            color: None,
            client_id: None,
            client_name: None,
            archived: 0,
            billable_default: 0,
        },
    ])
    .await
    .unwrap();

    let listed = r.list_projects().await.unwrap();
    assert_eq!(listed.len(), 2);
    let site = listed.iter().find(|p| p.id == "p-1").unwrap();
    let internal = listed.iter().find(|p| p.id == "p-2").unwrap();
    assert_eq!(site.client_name.as_deref(), Some("Acme"));
    assert_eq!(internal.client_name, None);
}
