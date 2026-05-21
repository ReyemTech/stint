//! Per-account refresh strategy.
//!
//! `refresh_account` orchestrates one provider call cycle: list calendars,
//! upsert them, then for every locally-included calendar pull events in
//! the given range and upsert them with a fresh `fetched_at`.
//!
//! `Ranges` builds the trigger-keyed time windows from spec §5.

use crate::calendar::provider::CalendarProvider;
use crate::calendar::store::CalendarStore;
use crate::calendar::types::{Calendar, CalendarEvent, TimeRange};
use crate::{time, Result};
use chrono::{Duration, Utc};

pub struct Ranges;

impl Ranges {
    /// Used when an account is first connected. Spec §5: last 7 + next 14.
    pub fn on_add() -> TimeRange {
        let now = Utc::now();
        TimeRange {
            start: now - Duration::days(7),
            end: now + Duration::days(14),
        }
    }

    /// Used on launch / main-window focus. Spec §5: next 7.
    pub fn on_focus() -> TimeRange {
        let now = Utc::now();
        TimeRange {
            start: now,
            end: now + Duration::days(7),
        }
    }

    /// Used by the periodic background poller. Spec §5: last 1 + next 7.
    pub fn background_poll() -> TimeRange {
        let now = Utc::now();
        TimeRange {
            start: now - Duration::days(1),
            end: now + Duration::days(7),
        }
    }
}

/// Pull a fresh snapshot of one account's calendars + events into the
/// store. Returns the number of events upserted across all included
/// calendars. Excluded calendars contribute 0.
pub async fn refresh_account(
    store: &CalendarStore,
    account_id: &str,
    provider: &dyn CalendarProvider,
    range: TimeRange,
) -> Result<usize> {
    // 1) Sync the calendar list.
    let remote_calendars = provider.list_calendars().await?;
    let calendars: Vec<Calendar> = remote_calendars
        .iter()
        .map(|c| Calendar {
            id: c.id.clone(),
            account_id: account_id.into(),
            name: c.name.clone(),
            color: c.color.clone(),
            included: true, // ignored by upsert; included is locality-preserved
            default_project_id: None, // local-only; never set from provider
        })
        .collect();
    store.upsert_calendars(account_id, &calendars).await?;

    // 2) For each locally-included calendar, pull events and upsert.
    let local_calendars = store.list_calendars(account_id).await?;
    let now = time::now_utc();
    let mut count = 0usize;

    for c in local_calendars.iter().filter(|c| c.included) {
        let events = provider.list_events(&c.id, range).await?;
        let to_upsert: Vec<CalendarEvent> = events
            .into_iter()
            .map(|e| CalendarEvent {
                id: e.id,
                account_id: account_id.into(),
                calendar_id: e.calendar_id,
                title: e.title,
                start_at: e.start_at,
                end_at: e.end_at,
                is_all_day: e.is_all_day,
                attendee_status: e.attendee_status,
                recurring_root: e.recurring_root,
                fetched_at: now.clone(),
            })
            .collect();
        count += to_upsert.len();
        if !to_upsert.is_empty() {
            store.upsert_events(&to_upsert).await?;
        }
    }
    Ok(count)
}

/// Refresh every enabled account under one range trigger. Used by the
/// background worker and the Tauri "Refresh now" command. Errors on one
/// account do not abort the others — each is captured and the highest-
/// priority error is returned at the end.
pub async fn refresh_all_enabled(
    store: &CalendarStore,
    providers: &[(&str, Box<dyn CalendarProvider>)],
    range: TimeRange,
) -> Result<usize> {
    let mut total = 0usize;
    let mut first_err: Option<crate::Error> = None;
    for (account_id, provider) in providers {
        match refresh_account(store, account_id, provider.as_ref(), range).await {
            Ok(n) => total += n,
            Err(e) => {
                tracing::warn!(account = %account_id, error = %e, "calendar refresh failed");
                first_err.get_or_insert(e);
            }
        }
    }
    if let Some(e) = first_err {
        return Err(e);
    }
    Ok(total)
}
