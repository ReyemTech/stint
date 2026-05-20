//! Google Calendar provider. Reuses `crate::oauth` for the PKCE flow and
//! `reqwest` for the v3 REST surface.

pub mod client;
pub mod config;
pub mod dto;

use crate::calendar::google::client::GoogleClient;
use crate::calendar::provider::{CalendarProvider, RemoteCalendar, RemoteEvent};
use crate::calendar::store::{calendar_blob_load, calendar_blob_save, CalendarOAuthBlob};
use crate::calendar::types::{ProviderKind, TimeRange};
use crate::config::secrets::Secrets;
use crate::oauth::client::OAuthClient;
use crate::solidtime::auth::{OAuthTokenProvider, PersistFn, TokenProvider};
use crate::{Error, Result};
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

/// Picks the user-facing identifier from a list of calendars returned by
/// `list_calendars`. Prefers the calendar with `primary == true` (Google's
/// primary calendar id is the user's email). Falls back to the first
/// calendar's name then id, and finally to a provided default.
pub fn resolve_account_identifier(
    cals: &[crate::calendar::provider::RemoteCalendar],
    default: &str,
) -> String {
    if let Some(p) = cals.iter().find(|c| c.primary) {
        // For Google, the primary entry's `id` is the user's email; `name`
        // is usually the email too. Prefer the id since it's the
        // canonical address.
        return p.id.clone();
    }
    cals.first()
        .map(|c| {
            if c.name.is_empty() {
                c.id.clone()
            } else {
                c.name.clone()
            }
        })
        .unwrap_or_else(|| default.to_string())
}

/// Build a fully-configured `GoogleProvider` for an account whose OAuth
/// credentials are stored in Keychain. This is the single shared entry
/// point for the Tauri command layer, the CLI subcommand, and the
/// background worker — they all need exactly this assembly:
/// load blob → construct OAuthClient with the blob's client_id/secret →
/// wire an OAuthTokenProvider whose `PersistFn` writes refreshed tokens
/// back to the same blob → wrap in a GoogleProvider with a fresh HTTP
/// client.
///
/// Returns `Error::MissingConfig("calendar.oauth")` when no blob exists
/// for the account.
pub fn build_provider_from_blob(
    secrets: &Secrets,
    account_id: &str,
) -> Result<Box<dyn CalendarProvider>> {
    let blob =
        calendar_blob_load(secrets, account_id)?.ok_or(Error::MissingConfig("calendar.oauth"))?;

    let mut cfg = config::google_oauth_config();
    cfg.client_id = blob.client_id.clone();
    if let Some(secret) = &blob.client_secret {
        cfg.client_secret = Some(secret.clone());
    }
    let oauth_client = OAuthClient::new(cfg);

    let secrets_clone = secrets.clone();
    let account_owned = account_id.to_string();
    let client_id_owned = blob.client_id.clone();
    let client_secret_owned = blob.client_secret.clone();
    let persist: PersistFn = Box::new(move |tokens| {
        let updated = CalendarOAuthBlob {
            client_id: client_id_owned.clone(),
            client_secret: client_secret_owned.clone(),
            tokens: tokens.clone(),
        };
        calendar_blob_save(&secrets_clone, &account_owned, &updated)
    });

    let tokens: Arc<dyn TokenProvider> =
        Arc::new(OAuthTokenProvider::new(oauth_client, blob.tokens, persist));
    Ok(Box::new(GoogleProvider::new(tokens, GoogleClient::new())))
}
