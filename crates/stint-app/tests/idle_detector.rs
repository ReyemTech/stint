//! Idle detector state machine (no actual CGEvent polling — that's tested
//! end-to-end via manual smoke).

use stint_app::idle_detector::{advance, IdleState};

#[test]
fn no_event_when_below_threshold() {
    let mut state = IdleState::default();
    let evt = advance(
        &mut state, /*idle_secs*/ 30.0, /*now*/ 1000, /*threshold*/ 600,
        /*timer_running*/ true,
    );
    assert!(evt.is_none());
    assert!(state.pending_idle.is_none());
}

#[test]
fn arms_pending_idle_when_threshold_reached() {
    let mut state = IdleState::default();
    // Idle for 720s when polled at t=1000 means idleness began at t=280
    let evt = advance(&mut state, 720.0, 1000, 600, true);
    assert!(evt.is_none());
    assert_eq!(state.pending_idle, Some(280));
}

#[test]
fn emits_event_when_activity_resumes() {
    let mut state = IdleState {
        pending_idle: Some(280),
    };
    let evt = advance(
        &mut state, /*idle_secs*/ 3.0, /*now*/ 1100, 600, true,
    );
    assert!(evt.is_some());
    let evt = evt.unwrap();
    assert_eq!(evt.idle_started, 280);
    assert_eq!(evt.idle_secs, 820); // now - pending_idle
    assert!(state.pending_idle.is_none());
}

#[test]
fn no_event_when_timer_not_running() {
    let mut state = IdleState::default();
    let evt = advance(&mut state, 720.0, 1000, 600, false);
    assert!(evt.is_none());
}

#[test]
fn drops_pending_when_timer_stops() {
    let mut state = IdleState {
        pending_idle: Some(280),
    };
    let evt = advance(&mut state, 3.0, 1100, 600, false);
    assert!(evt.is_none());
    assert!(state.pending_idle.is_none());
}
