mod common;

use stint_core::store::reference::{ProjectRow, Reference, TagRow, TaskRow};

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
            archived: 0,
        },
        ProjectRow {
            id: "p2".into(),
            name: "Reyem".into(),
            color: None,
            client_id: None,
            archived: 0,
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
        archived: 0,
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
        archived: 0,
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
