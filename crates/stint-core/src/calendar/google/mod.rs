//! Google Calendar provider. Reuses `crate::oauth` for the PKCE flow and
//! `reqwest` for the v3 REST surface.

pub mod client;
pub mod config;
pub mod dto;

use crate::calendar::google::client::GoogleClient;
use crate::calendar::provider::{CalendarProvider, RemoteCalendar, RemoteEvent};
use crate::calendar::types::{ProviderKind, TimeRange};
use crate::solidtime::auth::TokenProvider;
use crate::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// `CalendarProvider` implementation for Google Calendar. Owns an
/// `Arc<dyn TokenProvider>` so refresh logic — including persistence
/// back to Keychain — is shared with the Solidtime client.
pub struct GoogleProvider {
    tokens: Arc<dyn TokenProvider>,
    http: GoogleClient,
}

impl GoogleProvider {
    pub fn new(tokens: Arc<dyn TokenProvider>, http: GoogleClient) -> Self {
        Self { tokens, http }
    }
}

#[async_trait]
impl CalendarProvider for GoogleProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Google
    }

    async fn list_calendars(&self) -> Result<Vec<RemoteCalendar>> {
        let token = self.tokens.access_token().await?;
        self.http.list_calendars(&token).await
    }

    async fn list_events(&self, calendar_id: &str, range: TimeRange) -> Result<Vec<RemoteEvent>> {
        let token = self.tokens.access_token().await?;
        self.http.list_events(&token, calendar_id, range).await
    }
}
