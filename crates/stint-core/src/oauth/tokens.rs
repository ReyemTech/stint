//! Token bundle returned by an OAuth provider.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Safety margin around the absolute expiry — treat the token as expired this
/// long before the wire-reported expiry, to avoid TOCTOU on the wire.
const EXPIRY_SKEW: Duration = Duration::seconds(60);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub scope: Option<String>,
}

impl TokenSet {
    pub fn from_response(
        access_token: String,
        refresh_token: Option<String>,
        expires_in_seconds: i64,
        scope: Option<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            access_token,
            refresh_token,
            expires_at: now + Duration::seconds(expires_in_seconds),
            scope,
        }
    }

    /// True when `now + EXPIRY_SKEW >= expires_at`. Use to decide whether to
    /// refresh proactively.
    pub fn is_expired_with_skew(&self, now: DateTime<Utc>) -> bool {
        now + EXPIRY_SKEW >= self.expires_at
    }

    /// Apply a refresh response onto an existing TokenSet. Refresh-token from
    /// the response wins if present; otherwise the existing one is preserved
    /// (some providers only return refresh_token at initial issue).
    pub fn merge_refresh_response(
        &self,
        new_access_token: String,
        new_refresh_token: Option<String>,
        expires_in_seconds: i64,
        new_scope: Option<String>,
    ) -> Self {
        Self {
            access_token: new_access_token,
            refresh_token: new_refresh_token.or_else(|| self.refresh_token.clone()),
            expires_at: Utc::now() + Duration::seconds(expires_in_seconds),
            scope: new_scope.or_else(|| self.scope.clone()),
        }
    }
}
