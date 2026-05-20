use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use stint_core::calendar::provider::{CalendarProvider, RemoteCalendar, RemoteEvent};
use stint_core::calendar::types::{ProviderKind, TimeRange};
use stint_core::Result;

struct StubProvider {
    calendars: Vec<RemoteCalendar>,
    events: Vec<RemoteEvent>,
}

#[async_trait]
impl CalendarProvider for StubProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Google
    }
    async fn list_calendars(&self) -> Result<Vec<RemoteCalendar>> {
        Ok(self.calendars.clone())
    }
    async fn list_events(&self, _calendar_id: &str, _range: TimeRange) -> Result<Vec<RemoteEvent>> {
        Ok(self.events.clone())
    }
}

#[tokio::test]
async fn stub_provider_satisfies_trait() {
    let p = StubProvider {
        calendars: vec![RemoteCalendar {
            id: "primary".into(),
            name: "Primary".into(),
            color: Some("#000".into()),
            primary: false,
        }],
        events: vec![RemoteEvent {
            id: "evt-1".into(),
            calendar_id: "primary".into(),
            title: "Standup".into(),
            start_at: "2026-05-19T09:00:00Z".into(),
            end_at: "2026-05-19T09:15:00Z".into(),
            is_all_day: false,
            attendee_status: None,
            recurring_root: None,
        }],
    };
    assert_eq!(p.kind(), ProviderKind::Google);
    assert_eq!(p.list_calendars().await.unwrap().len(), 1);
    let range = TimeRange {
        start: Utc.with_ymd_and_hms(2026, 5, 19, 0, 0, 0).unwrap(),
        end: Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
    };
    let evs = p.list_events("primary", range).await.unwrap();
    assert_eq!(evs[0].title, "Standup");
}
