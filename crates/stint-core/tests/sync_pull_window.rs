use chrono::{Duration, TimeZone, Utc};
use stint_core::sync::pull::{Trigger, Window};

#[test]
fn window_for_on_startup_covers_last_24h() {
    let now = Utc.with_ymd_and_hms(2026, 5, 20, 17, 0, 0).unwrap();
    let w = Window::for_trigger(Trigger::OnStartup, now);
    assert_eq!(w.from, now - Duration::hours(24));
    assert_eq!(w.to, now);
}

#[test]
fn window_for_on_focus_covers_last_7d() {
    let now = Utc.with_ymd_and_hms(2026, 5, 20, 17, 0, 0).unwrap();
    let w = Window::for_trigger(Trigger::OnFocus, now);
    assert_eq!(w.from, now - Duration::days(7));
}

#[test]
fn window_for_background_poll_covers_last_1h() {
    let now = Utc.with_ymd_and_hms(2026, 5, 20, 17, 0, 0).unwrap();
    let w = Window::for_trigger(Trigger::BackgroundPoll, now);
    assert_eq!(w.from, now - Duration::hours(1));
}

#[test]
fn window_for_manual_covers_last_30d() {
    let now = Utc.with_ymd_and_hms(2026, 5, 20, 17, 0, 0).unwrap();
    let w = Window::for_trigger(Trigger::Manual, now);
    assert_eq!(w.from, now - Duration::days(30));
}
