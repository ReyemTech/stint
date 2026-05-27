//! Idle detector — polls CGEvent every 60s, emits an event on activity-
//! resume after the configured threshold has elapsed.
//!
//! The pure state machine in `advance()` is testable without macOS APIs;
//! the live polling loop (added in Task A4) calls `idle_seconds()` which
//! links against CoreGraphics.

use serde::Serialize;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct IdleState {
    /// Unix timestamp (seconds) when idleness began; Some once threshold
    /// has been reached and we're awaiting activity-resume.
    pub pending_idle: Option<u64>,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct IdleEvent {
    /// Epoch seconds when the idle period started.
    pub idle_started: u64,
    pub idle_secs: u64,
}

/// Advance the state machine one tick. Pure function; no I/O.
///
/// * `idle_secs` — CGEvent's "seconds since any input"
/// * `now` — current unix epoch seconds
/// * `threshold` — idle.threshold_secs setting
/// * `timer_running` — whether there's a running entry to attribute the gap to
pub fn advance(
    state: &mut IdleState,
    idle_secs: f64,
    now: u64,
    threshold: u32,
    timer_running: bool,
) -> Option<IdleEvent> {
    // No timer → nothing to attribute idle to. Drop any pending state.
    if !timer_running {
        state.pending_idle = None;
        return None;
    }

    let idle_secs = idle_secs.max(0.0) as u64;
    let threshold = threshold as u64;

    // Activity resumed after threshold was previously reached → emit.
    if let Some(idle_started) = state.pending_idle {
        if idle_secs < 60 {
            let evt = IdleEvent {
                idle_started,
                idle_secs: now.saturating_sub(idle_started),
            };
            state.pending_idle = None;
            return Some(evt);
        }
        // Still idle; no change.
        return None;
    }

    // Not yet armed. Arm if we crossed the threshold.
    if idle_secs >= threshold {
        state.pending_idle = Some(now.saturating_sub(idle_secs));
    }
    None
}

// ---- platform-dependent polling ----

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(source_state_id: i32, event_type: u32) -> f64;
}

/// Seconds since the last user input event (mouse / keyboard / etc).
/// macOS-only; on other platforms returns 0.0 (effectively disables the
/// detector).
#[cfg(target_os = "macos")]
pub fn idle_seconds() -> f64 {
    // source_state_id = 0 (combined session state),
    // event_type = u32::MAX (kCGAnyInputEventType)
    unsafe { CGEventSourceSecondsSinceLastEventType(0, u32::MAX) }
}

#[cfg(not(target_os = "macos"))]
pub fn idle_seconds() -> f64 {
    0.0
}

// The live polling loop (spawn) is added in Task A4.
