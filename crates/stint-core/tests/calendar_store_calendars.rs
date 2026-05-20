mod common;

use stint_core::calendar::store::CalendarStore;
use stint_core::calendar::types::{Calendar, CalendarAccount, ProviderKind};

async fn seed_account(s: &CalendarStore) {
    s.add_account(&CalendarAccount {
        id: "acc-1".into(),
        provider: ProviderKind::Google,
        display_name: "me@example.com".into(),
        identifier: "me@example.com".into(),
        caldav_url: None,
        enabled: true,
        created_at: "2026-05-19T00:00:00Z".into(),
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn upsert_calendars_replaces_set() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed_account(&s).await;

    s.upsert_calendars(
        "acc-1",
        &[
            Calendar {
                id: "primary".into(),
                account_id: "acc-1".into(),
                name: "Primary".into(),
                color: Some("#000".into()),
                included: true,
            },
            Calendar {
                id: "work".into(),
                account_id: "acc-1".into(),
                name: "Work".into(),
                color: None,
                included: true,
            },
        ],
    )
    .await
    .unwrap();

    let list = s.list_calendars("acc-1").await.unwrap();
    assert_eq!(list.len(), 2);

    // Rename "Primary" — included flag must survive the upsert.
    s.upsert_calendars(
        "acc-1",
        &[Calendar {
            id: "primary".into(),
            account_id: "acc-1".into(),
            name: "My Primary".into(),
            color: Some("#abc".into()),
            included: true, // value ignored by upsert; toggled via set_calendar_included
        }],
    )
    .await
    .unwrap();

    let list = s.list_calendars("acc-1").await.unwrap();
    let p = list.iter().find(|c| c.id == "primary").unwrap();
    assert_eq!(p.name, "My Primary");
    assert!(p.included, "included must not be clobbered by upsert");
}

#[tokio::test]
async fn set_calendar_included_toggles() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed_account(&s).await;
    s.upsert_calendars(
        "acc-1",
        &[Calendar {
            id: "primary".into(),
            account_id: "acc-1".into(),
            name: "Primary".into(),
            color: None,
            included: true,
        }],
    )
    .await
    .unwrap();
    s.set_calendar_included("primary", false).await.unwrap();
    let c = &s.list_calendars("acc-1").await.unwrap()[0];
    assert!(!c.included);
}

#[tokio::test]
async fn delete_account_cascades_to_calendars() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed_account(&s).await;
    s.upsert_calendars(
        "acc-1",
        &[Calendar {
            id: "primary".into(),
            account_id: "acc-1".into(),
            name: "Primary".into(),
            color: None,
            included: true,
        }],
    )
    .await
    .unwrap();
    s.delete_account("acc-1").await.unwrap();
    assert!(s.list_calendars("acc-1").await.unwrap().is_empty());
}
