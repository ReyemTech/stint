mod common;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use std::sync::Mutex;
use stint_core::calendar::provider::{CalendarProvider, RemoteCalendar, RemoteEvent};
use stint_core::calendar::store::CalendarStore;
use stint_core::calendar::sync::{refresh_account, refresh_all_enabled, Ranges};
use stint_core::calendar::types::{CalendarAccount, ProviderKind, TimeRange};
use stint_core::{Error, Result};

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
            primary: false,
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
                primary: false,
            },
            RemoteCalendar {
                id: "work".into(),
                name: "Work".into(),
                color: None,
                primary: false,
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

// --- helpers for the additional Ranges/refresh_all_enabled tests ---

fn scripted_with_one_event(cal_id: &str) -> ScriptedProvider {
    ScriptedProvider {
        calendars: vec![RemoteCalendar {
            id: cal_id.into(),
            name: cal_id.into(),
            color: None,
            primary: false,
        }],
        events_by_calendar: vec![(
            cal_id.into(),
            vec![RemoteEvent {
                id: format!("evt-{cal_id}"),
                calendar_id: cal_id.into(),
                title: "x".into(),
                start_at: "2026-05-19T09:00:00Z".into(),
                end_at: "2026-05-19T09:15:00Z".into(),
                is_all_day: false,
                attendee_status: None,
                recurring_root: None,
            }],
        )],
        last_range: Mutex::new(None),
    }
}

fn scripted_empty(cal_id: &str) -> ScriptedProvider {
    ScriptedProvider {
        calendars: vec![RemoteCalendar {
            id: cal_id.into(),
            name: cal_id.into(),
            color: None,
            primary: false,
        }],
        events_by_calendar: vec![(cal_id.into(), vec![])],
        last_range: Mutex::new(None),
    }
}

struct FailingProvider;

#[async_trait]
impl CalendarProvider for FailingProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Google
    }
    async fn list_calendars(&self) -> Result<Vec<RemoteCalendar>> {
        Err(Error::Invariant("simulated provider failure".into()))
    }
    async fn list_events(&self, _: &str, _: TimeRange) -> Result<Vec<RemoteEvent>> {
        Err(Error::Invariant("simulated provider failure".into()))
    }
}

#[tokio::test]
async fn refresh_account_passes_on_focus_range_to_provider() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed_account(&s, "acc-1").await;
    let provider = scripted_empty("primary");

    refresh_account(&s, "acc-1", &provider, Ranges::on_focus())
        .await
        .unwrap();

    let observed = provider
        .last_range
        .lock()
        .unwrap()
        .expect("provider.list_events should have been called");
    let now = Utc::now();
    // on_focus: now → now + 7d.
    assert!(observed.start <= now);
    assert!(observed.end > now + Duration::days(6));
    assert!(observed.end < now + Duration::days(8));
}

#[tokio::test]
async fn refresh_account_passes_background_range_to_provider() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed_account(&s, "acc-1").await;
    let provider = scripted_empty("primary");

    refresh_account(&s, "acc-1", &provider, Ranges::background_poll())
        .await
        .unwrap();

    let observed = provider
        .last_range
        .lock()
        .unwrap()
        .expect("provider.list_events should have been called");
    let now = Utc::now();
    // background_poll: last 1d, next 7d.
    assert!(observed.start < now - Duration::hours(20));
    assert!(observed.end > now + Duration::days(6));
    assert!(observed.end < now + Duration::days(8));
}

#[tokio::test]
async fn refresh_account_propagates_provider_error() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed_account(&s, "acc-1").await;

    let err = refresh_account(&s, "acc-1", &FailingProvider, Ranges::on_add())
        .await
        .unwrap_err();
    assert!(format!("{err}").contains("simulated provider failure"));
}

#[tokio::test]
async fn refresh_all_enabled_with_empty_providers_returns_zero() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    let providers: Vec<(&str, Box<dyn CalendarProvider>)> = vec![];

    let n = refresh_all_enabled(&s, &providers, Ranges::on_focus())
        .await
        .unwrap();
    assert_eq!(n, 0);
}

#[tokio::test]
async fn refresh_all_enabled_sums_counts_across_providers() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed_account(&s, "acc-1").await;
    seed_account(&s, "acc-2").await;

    let providers: Vec<(&str, Box<dyn CalendarProvider>)> = vec![
        ("acc-1", Box::new(scripted_with_one_event("c1"))),
        ("acc-2", Box::new(scripted_with_one_event("c2"))),
    ];

    let n = refresh_all_enabled(&s, &providers, Ranges::on_focus())
        .await
        .unwrap();
    assert_eq!(n, 2);
}

#[tokio::test]
async fn refresh_all_enabled_continues_after_error_but_returns_first_err() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed_account(&s, "acc-1").await;
    seed_account(&s, "acc-2").await;

    // Account 1 fails; account 2 must still run (single-account error doesn't
    // abort the loop) but the aggregate Result is Err.
    let healthy = scripted_with_one_event("c2");
    let providers: Vec<(&str, Box<dyn CalendarProvider>)> = vec![
        ("acc-1", Box::new(FailingProvider)),
        ("acc-2", Box::new(healthy)),
    ];

    let err = refresh_all_enabled(&s, &providers, Ranges::on_focus())
        .await
        .unwrap_err();
    assert!(format!("{err}").contains("simulated provider failure"));

    // Side-effect of account 2 still ran: a calendar row got persisted.
    let cals = s.list_calendars("acc-2").await.unwrap();
    assert_eq!(cals.len(), 1);
    assert_eq!(cals[0].id, "c2");
}
