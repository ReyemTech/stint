use stint_core::calendar::types::{
    AttendeeStatus, CalendarAccount, CalendarEvent, EventDecision, ProviderKind, TimeRange,
};
use chrono::{TimeZone, Utc};

#[test]
fn provider_kind_serde_roundtrip() {
    let kinds = [ProviderKind::Google];
    for k in kinds {
        let s = serde_json::to_string(&k).unwrap();
        let back: ProviderKind = serde_json::from_str(&s).unwrap();
        assert_eq!(k, back);
    }
}

#[test]
fn provider_kind_string_form_is_lowercase() {
    let s = serde_json::to_string(&ProviderKind::Google).unwrap();
    assert_eq!(s, "\"google\"");
}

#[test]
fn attendee_status_parses_known_values() {
    assert_eq!(AttendeeStatus::from_wire("accepted"), Some(AttendeeStatus::Accepted));
    assert_eq!(AttendeeStatus::from_wire("declined"), Some(AttendeeStatus::Declined));
    assert_eq!(AttendeeStatus::from_wire("tentative"), Some(AttendeeStatus::Tentative));
    assert_eq!(AttendeeStatus::from_wire("needsAction"), None);
    assert_eq!(AttendeeStatus::from_wire(""), None);
}

#[test]
fn event_decision_kind_serde() {
    let d = EventDecision::LoggedManual { linked_local_uuid: "uuid-1".into() };
    let s = serde_json::to_string(&d).unwrap();
    let back: EventDecision = serde_json::from_str(&s).unwrap();
    assert!(matches!(back, EventDecision::LoggedManual { .. }));
}

#[test]
fn time_range_inclusion_is_half_open() {
    let r = TimeRange {
        start: Utc.with_ymd_and_hms(2026, 5, 19, 9, 0, 0).unwrap(),
        end:   Utc.with_ymd_and_hms(2026, 5, 19, 10, 0, 0).unwrap(),
    };
    let at_start = Utc.with_ymd_and_hms(2026, 5, 19, 9, 0, 0).unwrap();
    let at_end   = Utc.with_ymd_and_hms(2026, 5, 19, 10, 0, 0).unwrap();
    let inside   = Utc.with_ymd_and_hms(2026, 5, 19, 9, 30, 0).unwrap();
    assert!(r.contains(at_start));
    assert!(!r.contains(at_end));   // half-open: [start, end)
    assert!(r.contains(inside));
}

#[test]
fn calendar_account_constructs_with_defaults() {
    let a = CalendarAccount {
        id: "acc-1".into(),
        provider: ProviderKind::Google,
        display_name: "me".into(),
        identifier: "me@example.com".into(),
        caldav_url: None,
        enabled: true,
        created_at: "2026-05-19T00:00:00Z".into(),
    };
    let _ = format!("{a:?}");   // ensure Debug is derived
    let s = serde_json::to_string(&a).unwrap();
    let back: CalendarAccount = serde_json::from_str(&s).unwrap();
    assert_eq!(back.identifier, "me@example.com");
}

#[test]
fn calendar_event_round_trips_with_optional_fields_absent() {
    let e = CalendarEvent {
        id: "evt-1".into(),
        account_id: "acc-1".into(),
        calendar_id: "cal-1".into(),
        title: "Standup".into(),
        start_at: "2026-05-19T09:00:00Z".into(),
        end_at: "2026-05-19T09:15:00Z".into(),
        is_all_day: false,
        attendee_status: None,
        recurring_root: None,
        fetched_at: "2026-05-19T00:00:00Z".into(),
    };
    let s = serde_json::to_string(&e).unwrap();
    let back: CalendarEvent = serde_json::from_str(&s).unwrap();
    assert_eq!(back.title, "Standup");
}
