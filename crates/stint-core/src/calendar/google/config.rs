//! OAuth config + scope constants for Google Calendar.
//!
//! `GOOGLE_OAUTH_CLIENT_ID` is non-secret — it's visible in every
//! authorize URL. PKCE protects the flow from interception. The
//! constant is overridable via `STINT_GOOGLE_CLIENT_ID` at runtime so
//! local dev and integration tests can inject a fake value without
//! editing source.

use crate::oauth::client::OAuthConfig;

pub const GOOGLE_AUTHORIZE_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const GOOGLE_CALENDAR_READONLY_SCOPE: &str =
    "https://www.googleapis.com/auth/calendar.readonly";

/// Default loopback redirect URI placeholder — `LoopbackServer` rewrites
/// the port at flow time, same as Solidtime's flow.
pub const GOOGLE_REDIRECT_URI_HOST: &str = "http://127.0.0.1:0/callback";

/// OAuth 2.0 client ID for the "stint desktop" application registered
/// on Google Cloud Console (Application type: "Desktop application").
/// Non-secret — visible in every authorize URL; PKCE protects the flow.
///
/// The `STINT_GOOGLE_CLIENT_ID` env var overrides this value at runtime
/// for integration tests and local dev against a different project.
pub const GOOGLE_OAUTH_CLIENT_ID: &str =
    "637936017220-u30p459mt9cb9fsqb22h6forn7svrb44.apps.googleusercontent.com";

/// Build a Google `OAuthConfig` using either the env-var override or
/// the baked-in client ID constant.
pub fn google_oauth_config() -> OAuthConfig {
    let client_id =
        std::env::var("STINT_GOOGLE_CLIENT_ID").unwrap_or_else(|_| GOOGLE_OAUTH_CLIENT_ID.into());
    google_oauth_config_with_client_id(&client_id)
}

pub fn google_oauth_config_with_client_id(client_id: &str) -> OAuthConfig {
    OAuthConfig {
        authorize_url: GOOGLE_AUTHORIZE_URL.into(),
        token_url: GOOGLE_TOKEN_URL.into(),
        client_id: client_id.into(),
        redirect_uri: GOOGLE_REDIRECT_URI_HOST.into(),
        scopes: vec![GOOGLE_CALENDAR_READONLY_SCOPE.into()],
        // Google needs both of these to consistently issue a refresh_token.
        extra_authorize_params: vec![
            ("access_type".into(), "offline".into()),
            ("prompt".into(), "consent".into()),
        ],
    }
}
