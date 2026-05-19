use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("keyring error: {0}")]
    Keyring(#[from] keyring::Error),

    #[error("solidtime API error: status {status}, body: {body}")]
    Solidtime { status: u16, body: String },

    #[error("solidtime auth failure (token invalid or revoked)")]
    SolidtimeAuth,

    #[error("solidtime config missing: {0}")]
    MissingConfig(&'static str),

    #[error("invariant violation: {0}")]
    Invariant(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("OAuth flow was cancelled or timed out")]
    OAuthCancelled,

    #[error("OAuth authorization server returned an error: {0}")]
    OAuthServer(String),

    #[error("OAuth refresh failed; user must re-authenticate")]
    OAuthRefreshFailed,

    #[error("OAuth state mismatch (possible CSRF)")]
    OAuthStateMismatch,

    #[error("Loopback redirect server failed to bind a port: {0}")]
    OAuthLoopback(String),
}

pub type Result<T> = std::result::Result<T, Error>;
