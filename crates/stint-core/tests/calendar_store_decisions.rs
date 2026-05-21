mod common;

use stint_core::calendar::store::CalendarStore;
use stint_core::calendar::types::{
    Calendar, CalendarAccount, CalendarEvent, EventDecision, ProviderKind,
};
use stint_core::store::Store;

/// Insert a minimal time_entries row so that linked_local_uuid FK constraints pass.
async fn insert_time_entry(store: &Store, local_uuid: &str) {
    sqlx::query(
        r#"INSERT INTO time_entries
           (local_uuid, description, start_at, source, sync_state, created_at, updated_at)
           VALUES (?, '', '2026-05-19T09:00:00Z', 'manual', 'pending',
                   '2026-05-19T00:00:00Z', '2026-05-19T00:00:00Z')"#,
    )
    .bind(local_uuid)
    .execute(store.pool())
    .await
    .unwrap();
}

async fn seed(s: &CalendarStore) {
    s.add_account(&CalendarAccount {
        id: "acc-1".into(),
        provider: ProviderKind::Google,
        display_name: "me".into(),
        identifier: "me@example.com".into(),
        caldav_url: None,
        enabled: true,
        created_at: "2026-05-19T00:00:00Z".into(),
    })
    .await
    .unwrap();
    s.upsert_calendars(
        "acc-1",
        &[Calendar {
            id: "primary".into(),
            account_id: "acc-1".into(),
            name: "Primary".into(),
            color: None,
            included: true,
            default_project_id: None,
        }],
    )
    .await
    .unwrap();
    s.upsert_events(&[CalendarEvent {
        id: "e1".into(),
        account_id: "acc-1".into(),
        calendar_id: "primary".into(),
        title: "Standup".into(),
        start_at: "2026-05-19T09:00:00Z".into(),
        end_at: "2026-05-19T09:15:00Z".into(),
        is_all_day: false,
        attendee_status: None,
        recurring_root: None,
        fetched_at: "2026-05-19T00:00:00Z".into(),
    }])
    .await
    .unwrap();
}

#[tokio::test]
async fn record_then_get_decision_returns_kind() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed(&s).await;

    s.record_decision(
        "acc-1",
        "e1",
        "2026-05-19T09:00:00Z",
        &EventDecision::Ignored,
    )
    .await
    .unwrap();

    let d = s
        .get_decision("acc-1", "e1", "2026-05-19T09:00:00Z")
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(d, EventDecision::Ignored));
}

#[tokio::test]
async fn record_decision_overwrites_previous() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed(&s).await;
    insert_time_entry(&env.store, "te-1").await;

    s.record_decision(
        "acc-1",
        "e1",
        "2026-05-19T09:00:00Z",
        &EventDecision::Ignored,
    )
    .await
    .unwrap();
    s.record_decision(
        "acc-1",
        "e1",
        "2026-05-19T09:00:00Z",
        &EventDecision::LoggedManual {
            linked_local_uuid: "te-1".into(),
        },
    )
    .await
    .unwrap();

    let d = s
        .get_decision("acc-1", "e1", "2026-05-19T09:00:00Z")
        .await
        .unwrap()
        .unwrap();
    match d {
        EventDecision::LoggedManual { linked_local_uuid } => {
            assert_eq!(linked_local_uuid, "te-1");
        }
        _ => panic!("expected LoggedManual"),
    }
}

#[tokio::test]
async fn list_decisions_filters_by_range() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed(&s).await;
    insert_time_entry(&env.store, "te-1").await;
    s.upsert_events(&[CalendarEvent {
        id: "e2".into(),
        account_id: "acc-1".into(),
        calendar_id: "primary".into(),
        title: "Next week".into(),
        start_at: "2026-05-25T09:00:00Z".into(),
        end_at: "2026-05-25T09:15:00Z".into(),
        is_all_day: false,
        attendee_status: None,
        recurring_root: None,
        fetched_at: "2026-05-19T00:00:00Z".into(),
    }])
    .await
    .unwrap();
    s.record_decision(
        "acc-1",
        "e1",
        "2026-05-19T09:00:00Z",
        &EventDecision::Ignored,
    )
    .await
    .unwrap();
    s.record_decision(
        "acc-1",
        "e2",
        "2026-05-25T09:00:00Z",
        &EventDecision::LoggedManual {
            linked_local_uuid: "te-1".into(),
        },
    )
    .await
    .unwrap();

    let list = s
        .list_decisions_in_range("acc-1", "2026-05-19T00:00:00Z", "2026-05-20T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].0, "e1");
}

#[tokio::test]
async fn clear_decision_deletes_existing_row() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed(&s).await;
    s.record_decision(
        "acc-1",
        "e1",
        "2026-05-19T09:00:00Z",
        &EventDecision::Ignored,
    )
    .await
    .unwrap();

    s.clear_decision("acc-1", "e1", "2026-05-19T09:00:00Z")
        .await
        .unwrap();

    let d = s
        .get_decision("acc-1", "e1", "2026-05-19T09:00:00Z")
        .await
        .unwrap();
    assert!(d.is_none(), "decision should be cleared");
}

#[tokio::test]
async fn clear_decision_is_idempotent_when_no_row_exists() {
    // Clearing a non-existent decision is a no-op rather than an error so
    // the revert flow stays trivial on the caller side.
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed(&s).await;

    s.clear_decision("acc-1", "e1", "2026-05-19T09:00:00Z")
        .await
        .unwrap();
}
