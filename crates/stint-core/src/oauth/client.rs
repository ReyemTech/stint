//! OAuthClient — provider-agnostic PKCE flow driver.
//!
//! `OAuthClient` does NOT itself open a browser or run the redirect server;
//! those concerns live in `crate::oauth::loopback` and the calling surface
//! (CLI / Tauri command). This module is testable in isolation.

use crate::oauth::pkce;
use crate::oauth::tokens::TokenSet;
use crate::{Error, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use rand::rngs::OsRng;
use rand::RngCore;
use reqwest::Client;
use serde::Deserialize;
use url::Url;

#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub authorize_url: String,
    pub token_url: String,
    pub client_id: String,
    /// Some providers (Google's "Installed" desktop clients) require a
    /// `client_secret` on the token endpoint even with PKCE. Per Google's
    /// docs the value isn't truly secret — it's bundled in the binary,
    /// paired with `client_id`. `None` for public clients (e.g. Solidtime
    /// Laravel Passport public client).
    pub client_secret: Option<String>,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    /// Provider-specific query params appended to the authorize URL.
    /// Empty for Solidtime; Google needs `access_type=offline` and
    /// `prompt=consent` to consistently issue a refresh_token.
    pub extra_authorize_params: Vec<(String, String)>,
}

pub struct OAuthClient {
    config: OAuthConfig,
}

pub struct PreparedAuthorize {
    pub authorize_url: Url,
    pub code_verifier: String,
    pub state: String,
}

impl OAuthClient {
    pub fn new(config: OAuthConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &OAuthConfig {
        &self.config
    }

    /// Build the authorize URL plus the PKCE verifier and CSRF state the caller
    /// must hold onto for the subsequent token exchange.
    pub fn prepare_authorize(&self) -> PreparedAuthorize {
        let code_verifier = pkce::generate_verifier();
        let code_challenge = pkce::code_challenge_for(&code_verifier);
        let state = random_state();

        let mut url = Url::parse(&self.config.authorize_url)
            .expect("authorize_url is a valid absolute URL (validated at config-load time)");
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.config.client_id)
            .append_pair("redirect_uri", &self.config.redirect_uri)
            .append_pair("scope", &self.config.scopes.join(" "))
            .append_pair("state", &state)
            .append_pair("code_challenge", &code_challenge)
            .append_pair("code_challenge_method", "S256");

        for (k, v) in &self.config.extra_authorize_params {
            url.query_pairs_mut().append_pair(k, v);
        }

        PreparedAuthorize {
            authorize_url: url,
            code_verifier,
            state,
        }
    }

    /// Exchange an authorization code for a `TokenSet` using the PKCE
    /// authorization_code grant.  POSTs an `application/x-www-form-urlencoded`
    /// body to `config.token_url` and parses the standard JSON response.
    pub async fn exchange_code(&self, code: &str, code_verifier: &str) -> Result<TokenSet> {
        let http = Client::new();
        let mut form: Vec<(&str, &str)> = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", self.config.redirect_uri.as_str()),
            ("client_id", self.config.client_id.as_str()),
            ("code_verifier", code_verifier),
        ];
        if let Some(s) = &self.config.client_secret {
            form.push(("client_secret", s.as_str()));
        }
        let resp = http
            .post(&self.config.token_url)
            .form(&form)
            .send()
            .await
            .map_err(|e| Error::OAuthServer(format!("token endpoint POST failed: {e}")))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| Error::OAuthServer(format!("token endpoint read failed: {e}")))?;

        if !status.is_success() {
            return Err(Error::OAuthServer(format!("HTTP {status}: {body}")));
        }

        let parsed: TokenResponse = serde_json::from_str(&body)
            .map_err(|e| Error::OAuthServer(format!("token endpoint JSON parse: {e}")))?;
        Ok(TokenSet::from_response(
            parsed.access_token,
            parsed.refresh_token,
            parsed.expires_in,
            parsed.scope,
            Utc::now(),
        ))
    }

    /// Exchange a stored refresh-token for a fresh `TokenSet` using the
    /// `refresh_token` grant.  If the server returns any 4xx status the
    /// refresh-token is considered dead and `Error::OAuthRefreshFailed` is
    /// returned so the caller can trigger re-authentication.
    pub async fn refresh_tokens(&self, prior: &TokenSet) -> Result<TokenSet> {
        let refresh_token = prior
            .refresh_token
            .as_deref()
            .ok_or(Error::OAuthRefreshFailed)?;

        let http = Client::new();
        let mut form: Vec<(&str, &str)> = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", self.config.client_id.as_str()),
        ];
        if let Some(s) = &self.config.client_secret {
            form.push(("client_secret", s.as_str()));
        }
        let resp = http
            .post(&self.config.token_url)
            .form(&form)
            .send()
            .await
            .map_err(|_| Error::OAuthRefreshFailed)?;

        let status = resp.status();
        // 4xx on refresh means the refresh_token is no good — require re-auth.
        if status.is_client_error() {
            return Err(Error::OAuthRefreshFailed);
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::OAuthServer(format!("HTTP {status}: {body}")));
        }
        let parsed: TokenResponse = resp.json().await.map_err(|_| Error::OAuthRefreshFailed)?;
        Ok(prior.merge_refresh_response(
            parsed.access_token,
            parsed.refresh_token,
            parsed.expires_in,
            parsed.scope,
        ))
    }
}

fn random_state() -> String {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
    scope: Option<String>,
}
