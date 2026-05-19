//! Provider-agnostic OAuth 2.0 (PKCE + authorization code) machinery.
//!
//! Used by `stint-core::solidtime::auth::OAuthTokenProvider` and (in
//! future phases) by calendar providers.

pub mod client;
pub mod loopback;
pub mod pkce;
pub mod tokens;