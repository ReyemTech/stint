mod common;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use std::sync::Mutex;
use stint_core::calendar::provider::{CalendarProvider, RemoteCalendar, RemoteEvent};
use stint_core::calendar::store::CalendarStore;
use stint_core::calendar::sync::{refresh_account, Ranges};
use stint_core::calendar::types::{CalendarAccount, ProviderKind, TimeRange};
use stint_core::Result;

struct ScriptedProvider {
    calendars: Vec<RemoteCalendar>,
    events_by_calendar: Vec<(String, Vec<RemoteEvent>)>,
    last_range: Mutex<Option<TimeRange>>,
}

#[async_trait]
impl CalendarProvider for ScriptedProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Google
    }
    async fn list_calendars(&self) -> Result<Vec<RemoteCalendar>> {
        Ok(self.calendars.clone())
    }
    async fn list_events(&self, calendar_id: &str, range: TimeRange) -> Result<Vec<RemoteEvent>> {
        *self.last_range.lock().unwrap() = Some(range);
        Ok(self
            .events_by_calendar
            .iter()
            .find(|(id, _)| id == calendar_id)
            .map(|(_, v)| v.clone())
            .unwrap_or_default())
    }
}

async fn seed_account(s: &CalendarStore, id: &str) {
    s.add_account(&CalendarAccount {
        id: id.into(),
        provider: ProviderKind::Google,
        display_name: "me".into(),
        identifier: "me@example.com".into(),
        caldav_url: None,
        enabled: true,
        created_at: "2026-05-19T00:00:00Z".into(),
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn refresh_account_inserts_calendars_and_events() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed_account(&s, "acc-1").await;

    let provider = ScriptedProvider {
        calendars: vec![RemoteCalendar {
            id: "primary".into(),
            name: "Primary".into(),
            color: None,
        }],
        events_by_calendar: vec![(
            "primary".into(),
            vec![RemoteEvent {
                id: "evt-1".into(),
                calendar_id: "primary".into(),
                title: "Standup".into(),
                start_at: "2026-05-19T09:00:00Z".into(),
                end_at: "2026-05-19T09:15:00Z".into(),
                is_all_day: false,
                attendee_status: None,
                recurring_root: None,
            }],
        )],
        last_range: Mutex::new(None),
    };

    let range = Ranges::on_add();
    let n = refresh_account(&s, "acc-1", &provider, range)
        .await
        .unwrap();
    assert_eq!(n, 1);
    let evs = s
        .list_events_in_range("acc-1", "2026-05-19T00:00:00Z", "2026-05-20T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(evs.len(), 1);
}

#[tokio::test]
async fn refresh_account_skips_excluded_calendars() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed_account(&s, "acc-1").await;

    let provider = ScriptedProvider {
        calendars: vec![
            RemoteCalendar {
                id: "primary".into(),
                name: "Primary".into(),
                color: None,
            },
            RemoteCalendar {
                id: "work".into(),
                name: "Work".into(),
                color: None,
            },
        ],
        events_by_calendar: vec![
            (
                "primary".into(),
                vec![RemoteEvent {
                    id: "evt-p".into(),
                    calendar_id: "primary".into(),
                    title: "p".into(),
                    start_at: "2026-05-19T09:00:00Z".into(),
                    end_at: "2026-05-19T09:15:00Z".into(),
                    is_all_day: false,
                    attendee_status: None,
                    recurring_root: None,
                }],
            ),
            (
                "work".into(),
                vec![RemoteEvent {
                    id: "evt-w".into(),
                    calendar_id: "work".into(),
                    title: "w".into(),
                    start_at: "2026-05-19T10:00:00Z".into(),
                    end_at: "2026-05-19T10:15:00Z".into(),
                    is_all_day: false,
                    attendee_status: None,
                    recurring_root: None,
                }],
            ),
        ],
        last_range: Mutex::new(None),
    };

    // First refresh imports both calendars. Then exclude "work".
    refresh_account(&s, "acc-1", &provider, Ranges::on_add())
        .await
        .unwrap();
    s.set_calendar_included("work", false).await.unwrap();

    // Subsequent refresh should not call list_events for "work" calendar.
    let n = refresh_account(&s, "acc-1", &provider, Ranges::on_add())
        .await
        .unwrap();
    assert_eq!(n, 1, "only the primary-calendar event should be returned");
}

#[tokio::test]
async fn ranges_on_add_spans_last_7_to_next_14_days() {
    let r = Ranges::on_add();
    let span = r.end - r.start;
    assert!(
        span >= Duration::days(20) && span <= Duration::days(22),
        "got {span}"
    );
    let now = Utc::now();
    assert!(r.start < now - Duration::days(6));
    assert!(r.end > now + Duration::days(13));
}

#[tokio::test]
async fn ranges_on_focus_spans_next_7_days() {
    let r = Ranges::on_focus();
    let now = Utc::now();
    assert!(r.start <= now);
    assert!(r.end > now + Duration::days(6));
    assert!(r.end < now + Duration::days(8));
}

#[tokio::test]
async fn ranges_background_spans_last_1_next_7() {
    let r = Ranges::background_poll();
    let now = Utc::now();
    assert!(r.start < now - Duration::hours(20));
    assert!(r.end > now + Duration::days(6));
}
