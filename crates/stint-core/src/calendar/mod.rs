//! Calendar integration — provider-agnostic types, store, and refresh
//! pipeline. Provider implementations live under submodules
//! (`google` ships in Phase 3b; `microsoft` and `caldav` are future).

pub mod google;
pub mod provider;
pub mod store;
pub mod sync;
pub mod types;

pub use provider::{CalendarProvider, RemoteCalendar, RemoteEvent};
pub use types::{
    AttendeeStatus, Calendar, CalendarAccount, CalendarEvent, EventDecision, ProviderKind, TimeRange,
};
