use chrono::{DateTime, Duration, Utc};

/// What caused this pull to fire. Determines the time window.
#[derive(Debug, Clone, Copy)]
pub enum Trigger {
    OnStartup,
    OnFocus,
    BackgroundPoll,
    Manual,
}

#[derive(Debug, Clone, Copy)]
pub struct Window {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

impl Window {
    pub fn for_trigger(trigger: Trigger, now: DateTime<Utc>) -> Self {
        let span = match trigger {
            Trigger::OnStartup => Duration::hours(24),
            Trigger::OnFocus => Duration::days(7),
            Trigger::BackgroundPoll => Duration::hours(1),
            Trigger::Manual => Duration::days(30),
        };
        Self {
            from: now - span,
            to: now,
        }
    }
}
