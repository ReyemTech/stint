mod common;

use stint_core::store::reference::{ClientRow, Reference};

#[tokio::test]
async fn upsert_then_list_clients_round_trips() {
    let env = common::setup().await;
    let r = Reference::new(env.store.clone());

    r.upsert_clients(&[
        ClientRow {
            id: "c-1".into(),
            name: "Acme".into(),
            archived: 0,
        },
        ClientRow {
            id: "c-2".into(),
            name: "Beta Co".into(),
            archived: 0,
        },
    ])
    .await
    .unwrap();

    let listed = r.list_clients().await.unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].name, "Acme");
    assert_eq!(listed[1].name, "Beta Co");
}

#[tokio::test]
async fn upsert_overwrites_existing_client() {
    let env = common::setup().await;
    let r = Reference::new(env.store.clone());

    r.upsert_clients(&[ClientRow {
        id: "c-1".into(),
        name: "Acme".into(),
        archived: 0,
    }])
    .await
    .unwrap();
    r.upsert_clients(&[ClientRow {
        id: "c-1".into(),
        name: "Acme Inc".into(),
        archived: 1,
    }])
    .await
    .unwrap();

    let listed = r.list_clients().await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "Acme Inc");
    assert_eq!(listed[0].archived, 1);
}
