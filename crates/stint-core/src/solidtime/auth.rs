//! Authentication providers for SolidtimeClient.
//!
//! A TokenProvider hands back a fresh bearer access-token on demand. Two
//! production impls: ApiTokenProvider (a static personal-access-token from
//! Keychain) and OAuthTokenProvider (refreshes on expiry using the
//! shared OAuth machinery). Tests use a mock impl directly.

use crate::config::secrets::Secrets;
use crate::oauth::client::OAuthClient;
use crate::oauth::tokens::TokenSet;
use crate::Result;
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[async_trait]
pub trait TokenProvider: Send + Sync {
    async fn access_token(&self) -> Result<String>;
}

/// Static personal-access-token. Used when `solidtime.auth_mode = "api_token"`.
pub struct ApiTokenProvider {
    token: String,
}

impl ApiTokenProvider {
    pub fn new(token: String) -> Self {
        Self { token }
    }
}

#[async_trait]
impl TokenProvider for ApiTokenProvider {
    async fn access_token(&self) -> Result<String> {
        Ok(self.token.clone())
    }
}

pub type PersistFn = Box<dyn Fn(&TokenSet) -> Result<()> + Send + Sync>;

pub struct OAuthTokenProvider {
    client: OAuthClient,
    state: Mutex<TokenSet>,
    persist: PersistFn,
}

impl OAuthTokenProvider {
    pub fn new(client: OAuthClient, initial: TokenSet, persist: PersistFn) -> Self {
        Self {
            client,
            state: Mutex::new(initial),
            persist,
        }
    }
}

#[async_trait]
impl TokenProvider for OAuthTokenProvider {
    async fn access_token(&self) -> Result<String> {
        // Cheap check: read current state, decide if refresh is needed.
        let needs_refresh = {
            let s = self.state.lock().unwrap();
            s.is_expired_with_skew(Utc::now())
        };

        if needs_refresh {
            let prior = { self.state.lock().unwrap().clone() };
            let refreshed = self.client.refresh_tokens(&prior).await?;
            (self.persist)(&refreshed)?;
            let mut guard = self.state.lock().unwrap();
            *guard = refreshed;
        }

        let guard = self.state.lock().unwrap();
        Ok(guard.access_token.clone())
    }
}

const OAUTH_KEYCHAIN_KEY: &str = "solidtime.oauth";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthBlob {
    pub client_id: String,
    pub tokens: TokenSet,
}

pub fn oauth_blob_load(secrets: &Secrets) -> Result<Option<OAuthBlob>> {
    let Some(raw) = secrets.get(OAUTH_KEYCHAIN_KEY)? else {
        return Ok(None);
    };
    let blob: OAuthBlob = serde_json::from_str(&raw)
        .map_err(|e| crate::Error::OAuthServer(format!("OAuth Keychain blob malformed: {e}")))?;
    Ok(Some(blob))
}

pub fn oauth_blob_save(secrets: &Secrets, blob: &OAuthBlob) -> Result<()> {
    let raw = serde_json::to_string(blob).expect("OAuthBlob is JSON-serializable");
    secrets.set(OAUTH_KEYCHAIN_KEY, &raw)
}

pub fn oauth_blob_delete(secrets: &Secrets) -> Result<()> {
    secrets.delete(OAUTH_KEYCHAIN_KEY)
}
