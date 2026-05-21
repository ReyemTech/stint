mod common;

use stint_core::calendar::store::CalendarStore;
use stint_core::calendar::types::{Calendar, CalendarAccount, CalendarEvent, ProviderKind};

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
}

fn evt(id: &str, start: &str, end: &str, title: &str) -> CalendarEvent {
    CalendarEvent {
        id: id.into(),
        account_id: "acc-1".into(),
        calendar_id: "primary".into(),
        title: title.into(),
        start_at: start.into(),
        end_at: end.into(),
        is_all_day: false,
        attendee_status: None,
        recurring_root: None,
        fetched_at: "2026-05-19T00:00:00Z".into(),
    }
}

#[tokio::test]
async fn upsert_then_list_returns_events_sorted_by_start() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed(&s).await;

    s.upsert_events(&[
        evt(
            "e2",
            "2026-05-19T11:00:00Z",
            "2026-05-19T11:30:00Z",
            "Lunch prep",
        ),
        evt(
            "e1",
            "2026-05-19T09:00:00Z",
            "2026-05-19T09:15:00Z",
            "Standup",
        ),
    ])
    .await
    .unwrap();

    let list = s
        .list_events_in_range("acc-1", "2026-05-19T00:00:00Z", "2026-05-20T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, "e1");
    assert_eq!(list[1].id, "e2");
}

#[tokio::test]
async fn upsert_is_idempotent_for_same_key() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed(&s).await;

    let e = evt(
        "e1",
        "2026-05-19T09:00:00Z",
        "2026-05-19T09:15:00Z",
        "Standup",
    );
    s.upsert_events(std::slice::from_ref(&e)).await.unwrap();
    let e2 = CalendarEvent {
        title: "Standup (renamed)".into(),
        ..e.clone()
    };
    s.upsert_events(&[e2]).await.unwrap();

    let list = s
        .list_events_in_range("acc-1", "2026-05-19T00:00:00Z", "2026-05-20T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].title, "Standup (renamed)");
}

#[tokio::test]
async fn recurring_instances_at_different_starts_coexist() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed(&s).await;
    s.upsert_events(&[
        evt(
            "recurring",
            "2026-05-19T09:00:00Z",
            "2026-05-19T09:15:00Z",
            "Standup",
        ),
        evt(
            "recurring",
            "2026-05-26T09:00:00Z",
            "2026-05-26T09:15:00Z",
            "Standup",
        ),
    ])
    .await
    .unwrap();
    let list = s
        .list_events_in_range("acc-1", "2026-05-19T00:00:00Z", "2026-06-01T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn list_events_in_range_excludes_outside_window() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed(&s).await;
    s.upsert_events(&[
        evt(
            "e1",
            "2026-05-19T09:00:00Z",
            "2026-05-19T09:15:00Z",
            "Standup",
        ),
        evt(
            "e2",
            "2026-05-25T09:00:00Z",
            "2026-05-25T09:15:00Z",
            "Future",
        ),
    ])
    .await
    .unwrap();
    let list = s
        .list_events_in_range("acc-1", "2026-05-19T00:00:00Z", "2026-05-20T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "e1");
}

#[tokio::test]
async fn list_events_in_range_excludes_calendars_not_included() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed(&s).await;
    // Add a second calendar, then exclude the primary.
    s.upsert_calendars(
        "acc-1",
        &[Calendar {
            id: "extra".into(),
            account_id: "acc-1".into(),
            name: "Extra".into(),
            color: None,
            included: true,
            default_project_id: None,
        }],
    )
    .await
    .unwrap();
    s.upsert_events(&[
        evt(
            "e1",
            "2026-05-19T09:00:00Z",
            "2026-05-19T09:15:00Z",
            "From primary",
        ),
        CalendarEvent {
            id: "e2".into(),
            calendar_id: "extra".into(),
            ..evt(
                "e2",
                "2026-05-19T10:00:00Z",
                "2026-05-19T10:15:00Z",
                "From extra",
            )
        },
    ])
    .await
    .unwrap();
    s.set_calendar_included("primary", false).await.unwrap();

    let list = s
        .list_events_in_range("acc-1", "2026-05-19T00:00:00Z", "2026-05-20T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "e2");
}
