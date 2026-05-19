//! Provider-agnostic calendar interface. `calendar::sync` is written
//! against this trait so MS Graph (Phase 3c) and CalDAV (Phase 3d)
//! can plug in without disturbing the refresher.

use crate::calendar::types::{AttendeeStatus, ProviderKind, TimeRange};
use crate::Result;
use async_trait::async_trait;

#[async_trait]
pub trait CalendarProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    async fn list_calendars(&self) -> Result<Vec<RemoteCalendar>>;
    async fn list_events(
        &self,
        calendar_id: &str,
        range: TimeRange,
    ) -> Result<Vec<RemoteEvent>>;
}

/// Provider-shaped calendar — same fields as the domain `Calendar`, minus
/// the `account_id` (assigned at upsert time) and `included` flag (a local
/// concept, not part of the remote view).
#[derive(Debug, Clone)]
pub struct RemoteCalendar {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
}

/// Provider-shaped event — domain `CalendarEvent` minus `account_id` and
/// `fetched_at` (assigned at upsert time).
#[derive(Debug, Clone)]
pub struct RemoteEvent {
    pub id: String,
    pub calendar_id: String,
    pub title: String,
    pub start_at: String,
    pub end_at: String,
    pub is_all_day: bool,
    pub attendee_status: Option<AttendeeStatus>,
    pub recurring_root: Option<String>,
}
