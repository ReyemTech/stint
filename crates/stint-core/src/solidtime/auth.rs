//! Authentication providers for SolidtimeClient.
//!
//! A TokenProvider hands back a fresh bearer access-token on demand. Two
//! production impls: ApiTokenProvider (a static personal-access-token from
//! Keychain) and OAuthTokenProvider (refreshes on expiry using the
//! shared OAuth machinery). Tests use a mock impl directly.

use crate::config::secrets::Secrets;
use crate::config::Settings;
use crate::oauth::client::{OAuthClient, OAuthConfig};
use crate::oauth::loopback::listen_for_callback;
use crate::oauth::tokens::TokenSet;
use crate::Result;
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;

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

/// Run the full PKCE flow: spin up a loopback server, mutate the redirect_uri
/// in `client.config` to include the bound port, generate authorize URL, call
/// `open_browser(authorize_url_string)`, await the callback, exchange the code,
/// return the TokenSet. The caller persists the TokenSet.
pub async fn login_interactive<F>(
    client: &OAuthClient,
    flow_timeout: Duration,
    open_browser: F,
) -> Result<TokenSet>
where
    F: FnOnce(String),
{
    let server = listen_for_callback(flow_timeout).await?;
    let port = server.port();
    let mut config = client.config().clone();
    config.redirect_uri = format!("http://127.0.0.1:{port}/callback");
    let runtime_client = OAuthClient::new(config);

    let prepared = runtime_client.prepare_authorize();
    open_browser(prepared.authorize_url.to_string());

    let captured = server.await_callback().await?;
    if captured.state != prepared.state {
        return Err(crate::Error::OAuthStateMismatch);
    }

    runtime_client
        .exchange_code(&captured.code, &prepared.code_verifier)
        .await
}

const AUTH_MODE_KEY: &str = "solidtime.auth_mode";
const API_TOKEN_KEYCHAIN_KEY: &str = "solidtime.token";

// Empty by default. Passport instances without `Passport::tokensCan(...)`
// configured reject explicit scope requests with `invalid_scope`. Scopes are
// not currently enforced by Solidtime (SECURITY.md flags this as a known gap),
// so the default token covers all operations.
const DEFAULT_SCOPES: &[&str] = &[];
const DEFAULT_REDIRECT_URI_HOST: &str = "http://127.0.0.1:0/callback";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMode {
    ApiToken,
    OAuth,
}

impl AuthMode {
    pub fn from_str_or_default(s: Option<&str>) -> Self {
        match s {
            Some("oauth") => Self::OAuth,
            _ => Self::ApiToken,
        }
    }
}

/// Build the right `(TokenProvider, OAuthClient)` pair based on settings + Keychain.
/// The OAuthClient is returned even for the api_token path so the GUI can offer a
/// "Sign in with Solidtime" button without re-resolving config.
pub async fn build_token_provider(
    settings: &Settings,
    secrets: &Secrets,
    solidtime_base_url: &str,
) -> Result<(Arc<dyn TokenProvider>, OAuthClient)> {
    let mode = AuthMode::from_str_or_default(settings.get(AUTH_MODE_KEY).await?.as_deref());

    let blob = oauth_blob_load(secrets)?;
    let client_id = blob
        .as_ref()
        .map(|b| b.client_id.clone())
        .unwrap_or_else(|| "stint-desktop".to_string());

    let oauth_client = OAuthClient::new(OAuthConfig {
        authorize_url: format!(
            "{}/oauth/authorize",
            solidtime_base_url.trim_end_matches('/')
        ),
        token_url: format!("{}/oauth/token", solidtime_base_url.trim_end_matches('/')),
        client_id,
        redirect_uri: DEFAULT_REDIRECT_URI_HOST.into(),
        scopes: DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect(),
    });

    match mode {
        AuthMode::ApiToken => {
            let token = secrets
                .get(API_TOKEN_KEYCHAIN_KEY)?
                .ok_or(crate::Error::MissingConfig("solidtime.token"))?;
            let provider: Arc<dyn TokenProvider> = Arc::new(ApiTokenProvider::new(token));
            Ok((provider, oauth_client))
        }
        AuthMode::OAuth => {
            let blob = blob.ok_or(crate::Error::MissingConfig("solidtime.oauth"))?;
            let secrets_clone = secrets.clone();
            let persist: PersistFn = Box::new(move |t: &TokenSet| {
                let updated = OAuthBlob {
                    client_id: blob.client_id.clone(),
                    tokens: t.clone(),
                };
                oauth_blob_save(&secrets_clone, &updated)
            });
            let provider: Arc<dyn TokenProvider> = Arc::new(OAuthTokenProvider::new(
                OAuthClient::new(oauth_client.config().clone()),
                blob.tokens,
                persist,
            ));
            Ok((provider, oauth_client))
        }
    }
}
