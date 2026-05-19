//! Authentication providers for SolidtimeClient.
//!
//! A TokenProvider hands back a fresh bearer access-token on demand. Two
//! production impls: ApiTokenProvider (a static personal-access-token from
//! Keychain) and OAuthTokenProvider (refreshes on expiry using the
//! shared OAuth machinery). Tests use a mock impl directly.

use crate::Result;
use async_trait::async_trait;

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
