//! OAuth config + scope constants for Google Calendar.
//!
//! Credentials (`GOOGLE_OAUTH_CLIENT_ID` and `GOOGLE_OAUTH_CLIENT_SECRET`)
//! are baked in at compile time via the `STINT_GOOGLE_CLIENT_ID` and
//! `STINT_GOOGLE_CLIENT_SECRET` environment variables. Release builds set
//! these in the build environment; forks that don't set them build fine
//! but get an empty string at runtime, which Google rejects with
//! `invalid_client` — see `is_configured()` for a pre-flight check the
//! UI can use to render a clearer message.
//!
//! Per Google's docs, the "client_secret" for an Installed/Desktop client
//! is bundled in the binary alongside the client_id — required at the
//! token endpoint even with PKCE. Stint follows the same pattern but
//! keeps both out of the public repo so forkers can't reuse stint's
//! Google Cloud project quota.

use crate::oauth::client::OAuthConfig;

pub const GOOGLE_AUTHORIZE_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const GOOGLE_CALENDAR_READONLY_SCOPE: &str =
    "https://www.googleapis.com/auth/calendar.readonly";

pub const GOOGLE_REDIRECT_URI_HOST: &str = "http://127.0.0.1:0/callback";

/// OAuth 2.0 client ID for the registered "stint desktop" application.
/// Baked at compile time from `STINT_GOOGLE_CLIENT_ID`; empty if unset.
pub const GOOGLE_OAUTH_CLIENT_ID: &str = match option_env!("STINT_GOOGLE_CLIENT_ID") {
    Some(s) => s,
    None => "",
};

/// OAuth 2.0 client "secret" for the Desktop application. Per Google's
/// docs this is bundled in the binary — not truly secret — but kept out
/// of the public repo so forkers register their own Google Cloud project.
/// Baked at compile time from `STINT_GOOGLE_CLIENT_SECRET`; empty if unset.
pub const GOOGLE_OAUTH_CLIENT_SECRET: &str = match option_env!("STINT_GOOGLE_CLIENT_SECRET") {
    Some(s) => s,
    None => "",
};

/// Returns true if both client_id and client_secret were baked into this
/// binary at compile time. Callers should check before initiating a
/// Google OAuth flow to render a clearer "not configured" message.
pub fn is_configured() -> bool {
    !GOOGLE_OAUTH_CLIENT_ID.is_empty() && !GOOGLE_OAUTH_CLIENT_SECRET.is_empty()
}

pub fn google_oauth_config() -> OAuthConfig {
    google_oauth_config_with_client_id(GOOGLE_OAUTH_CLIENT_ID)
}

pub fn google_oauth_config_with_client_id(client_id: &str) -> OAuthConfig {
    OAuthConfig {
        authorize_url: GOOGLE_AUTHORIZE_URL.into(),
        token_url: GOOGLE_TOKEN_URL.into(),
        client_id: client_id.into(),
        client_secret: if GOOGLE_OAUTH_CLIENT_SECRET.is_empty() {
            None
        } else {
            Some(GOOGLE_OAUTH_CLIENT_SECRET.into())
        },
        redirect_uri: GOOGLE_REDIRECT_URI_HOST.into(),
        scopes: vec![GOOGLE_CALENDAR_READONLY_SCOPE.into()],
        extra_authorize_params: vec![
            ("access_type".into(), "offline".into()),
            ("prompt".into(), "consent".into()),
        ],
    }
}
