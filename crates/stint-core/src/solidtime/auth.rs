//! Authentication providers for SolidtimeClient.
//!
//! A TokenProvider hands back a fresh bearer access-token on demand. Two
//! production impls: ApiTokenProvider (a static personal-access-token from
//! Keychain) and OAuthTokenProvider (refreshes on expiry using the
//! shared OAuth machinery). Tests use a mock impl directly.

use crate::oauth::client::OAuthClient;
use crate::oauth::tokens::TokenSet;
use crate::Result;
use async_trait::async_trait;
use chrono::Utc;
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
