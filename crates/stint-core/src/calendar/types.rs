//! Calendar domain types — shared by the provider trait, the store,
//! the sync refresher, and the public API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Google,
    // Phase 3c: Microsoft
    // Phase 3d: CalDav
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AttendeeStatus {
    Accepted,
    Declined,
    Tentative,
}

impl AttendeeStatus {
    /// Map the provider's on-the-wire string to a known status. Returns
    /// `None` for values we do not normalize (e.g. Google's `needsAction`).
    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "accepted" => Some(Self::Accepted),
            "declined" => Some(Self::Declined),
            "tentative" => Some(Self::Tentative),
            _ => None,
        }
    }

    pub fn as_wire(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Declined => "declined",
            Self::Tentative => "tentative",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarAccount {
    pub id: String, // local uuid
    pub provider: ProviderKind,
    pub display_name: String,
    pub identifier: String, // email for OAuth providers
    pub caldav_url: Option<String>,
    pub enabled: bool,
    pub created_at: String, // RFC 3339
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calendar {
    pub id: String, // provider-native id
    pub account_id: String,
    pub name: String,
    pub color: Option<String>,
    pub included: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String, // provider-native id
    pub account_id: String,
    pub calendar_id: String,
    pub title: String,
    pub start_at: String, // RFC 3339 (or YYYY-MM-DD for all-day)
    pub end_at: String,
    pub is_all_day: bool,
    pub attendee_status: Option<AttendeeStatus>,
    pub recurring_root: Option<String>,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventDecision {
    Ignored,
    LoggedManual { linked_local_uuid: String },
    LoggedAuto { linked_local_uuid: String },
}

impl EventDecision {
    /// Returns the decision string stored in the `event_decisions.decision`
    /// column. Symmetric with [`Self::decoded`].
    pub fn as_wire(&self) -> &'static str {
        match self {
            Self::Ignored => "ignored",
            Self::LoggedManual { .. } => "logged_manual",
            Self::LoggedAuto { .. } => "logged_auto",
        }
    }

    pub fn linked_local_uuid(&self) -> Option<&str> {
        match self {
            Self::Ignored => None,
            Self::LoggedManual { linked_local_uuid } | Self::LoggedAuto { linked_local_uuid } => {
                Some(linked_local_uuid)
            }
        }
    }

    pub fn decoded(wire: &str, linked_local_uuid: Option<String>) -> Option<Self> {
        match (wire, linked_local_uuid) {
            ("ignored", _) => Some(Self::Ignored),
            ("logged_manual", Some(uuid)) => Some(Self::LoggedManual {
                linked_local_uuid: uuid,
            }),
            ("logged_auto", Some(uuid)) => Some(Self::LoggedAuto {
                linked_local_uuid: uuid,
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl TimeRange {
    /// Half-open `[start, end)`. Useful for both "today" queries and refresh-window logic.
    pub fn contains(&self, t: DateTime<Utc>) -> bool {
        t >= self.start && t < self.end
    }
}
