//! HTTP wrapper over Google Calendar v3.
//!
//! Only the read-side endpoints we care about for Phase 3b: list user's
//! `calendarList` and per-calendar `events` (with `singleEvents=true`
//! server-side expansion of recurrences and `pageToken` paging).

use crate::calendar::google::dto::{CalendarListResponse, EventsResponse};
use crate::calendar::provider::{RemoteCalendar, RemoteEvent};
use crate::calendar::types::TimeRange;
use crate::{Error, Result};
use reqwest::{Client, StatusCode};

pub const GOOGLE_API_BASE: &str = "https://www.googleapis.com";

pub struct GoogleClient {
    base_url: String,
    http: Client,
}

impl GoogleClient {
    pub fn new() -> Self {
        Self::with_base_url(GOOGLE_API_BASE)
    }

    pub fn with_base_url(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("reqwest client builds"),
        }
    }

    pub async fn list_calendars(&self, access_token: &str) -> Result<Vec<RemoteCalendar>> {
        let url = format!("{}/calendar/v3/users/me/calendarList", self.base_url);
        let resp = self
            .http
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(Error::from)?;
        let status = resp.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err(Error::OAuthRefreshFailed);
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::OAuthServer(format!(
                "google calendarList HTTP {status}: {body}"
            )));
        }
        let parsed: CalendarListResponse = resp.json().await?;
        Ok(parsed.items.into_iter().map(|c| c.into_remote()).collect())
    }

    pub async fn list_events(
        &self,
        access_token: &str,
        calendar_id: &str,
        range: TimeRange,
    ) -> Result<Vec<RemoteEvent>> {
        let mut events = Vec::new();
        let mut page_token: Option<String> = None;
        let time_min = range.start.to_rfc3339();
        let time_max = range.end.to_rfc3339();

        loop {
            let url = format!(
                "{}/calendar/v3/calendars/{}/events",
                self.base_url,
                urlencoding::encode(calendar_id)
            );
            let mut req = self.http.get(&url).bearer_auth(access_token).query(&[
                ("singleEvents", "true"),
                ("orderBy", "startTime"),
                ("timeMin", time_min.as_str()),
                ("timeMax", time_max.as_str()),
                ("maxResults", "250"),
            ]);
            if let Some(tok) = &page_token {
                req = req.query(&[("pageToken", tok.as_str())]);
            }
            let resp = req.send().await.map_err(Error::from)?;
            let status = resp.status();
            if status == StatusCode::UNAUTHORIZED {
                return Err(Error::OAuthRefreshFailed);
            }
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(Error::OAuthServer(format!(
                    "google events HTTP {status}: {body}"
                )));
            }
            let parsed: EventsResponse = resp.json().await?;
            for item in parsed.items {
                events.push(item.into_remote(calendar_id));
            }
            match parsed.next_page_token {
                Some(tok) if !tok.is_empty() => page_token = Some(tok),
                _ => break,
            }
        }
        Ok(events)
    }
}

impl Default for GoogleClient {
    fn default() -> Self {
        Self::new()
    }
}
