//! Wire DTOs for the Google Calendar v3 REST surface, plus mappers to
//! the provider-shaped `RemoteCalendar` / `RemoteEvent`.

use crate::calendar::provider::{RemoteCalendar, RemoteEvent};
use crate::calendar::types::AttendeeStatus;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct CalendarListResponse {
    #[serde(default)]
    pub items: Vec<CalendarListEntry>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CalendarListEntry {
    pub id: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default, rename = "backgroundColor")]
    pub background_color: Option<String>,
    #[serde(default)]
    pub primary: bool,
}

impl CalendarListEntry {
    pub(crate) fn into_remote(self) -> RemoteCalendar {
        RemoteCalendar {
            id: self.id,
            name: self.summary.unwrap_or_default(),
            color: self.background_color,
            primary: self.primary,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct EventsResponse {
    #[serde(default)]
    pub items: Vec<EventEntry>,
    #[serde(default, rename = "nextPageToken")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EventEntry {
    pub id: String,
    #[serde(default)]
    pub summary: Option<String>,
    pub start: EventTime,
    pub end: EventTime,
    #[serde(default, rename = "recurringEventId")]
    pub recurring_event_id: Option<String>,
    #[serde(default)]
    pub attendees: Vec<EventAttendee>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EventTime {
    #[serde(default)]
    pub date: Option<String>, // YYYY-MM-DD for all-day events
    #[serde(default, rename = "dateTime")]
    pub date_time: Option<String>, // RFC 3339 for timed events
}

#[derive(Debug, Deserialize)]
pub(crate) struct EventAttendee {
    #[serde(default, rename = "self")]
    pub is_self: bool,
    #[serde(default, rename = "responseStatus")]
    pub response_status: Option<String>,
}

impl EventEntry {
    pub(crate) fn into_remote(self, calendar_id: &str) -> RemoteEvent {
        let is_all_day = self.start.date.is_some();
        let start_at = self.start.date_time.or(self.start.date).unwrap_or_default();
        let end_at = self.end.date_time.or(self.end.date).unwrap_or_default();
        let attendee_status = self
            .attendees
            .iter()
            .find(|a| a.is_self)
            .and_then(|a| a.response_status.as_deref())
            .and_then(AttendeeStatus::from_wire);

        RemoteEvent {
            id: self.id,
            calendar_id: calendar_id.into(),
            title: self.summary.unwrap_or_default(),
            start_at,
            end_at,
            is_all_day,
            attendee_status,
            recurring_root: self.recurring_event_id,
        }
    }
}
