# stint Phase 3a: OAuth 2.0 Foundation + Solidtime OAuth Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a shared OAuth 2.0 PKCE machinery to `stint-core` and ship Solidtime OAuth as the first user-facing consumer alongside the existing API-token path. After this phase, a user can pick API-token or OAuth in Settings; both work; OAuth refresh-tokens transparently. The shared machinery (PKCE primitives, loopback redirect server, token refresh loop, Keychain persistence) is positioned for re-use by Phase 3b/c (Google + Microsoft calendars).

**Architecture:**
- New module `stint-core/src/oauth/` contains the provider-agnostic OAuth machinery. It depends on the `oauth2` crate for protocol mechanics and on `keyring` for token persistence, but knows nothing about Solidtime / Google / Microsoft.
- New `TokenProvider` trait in `stint-core/src/solidtime/auth.rs` abstracts "give me a fresh access token." Two implementations: `ApiTokenProvider` (returns the stored static token) and `OAuthTokenProvider` (returns the cached access token, refreshing via the shared OAuth machinery when it has expired). `SolidtimeClient` is refactored to hold `Arc<dyn TokenProvider>` instead of `token: String`, so every API call gets a fresh token automatically.
- Settings table gains one new key: `solidtime.auth_mode` ∈ `{"api_token", "oauth"}`. Default `api_token` (preserves existing behaviour for anyone with a token configured). OAuth refresh-token + access-token + expiry are stored in Keychain as a single JSON blob under `tech.reyem.stint.solidtime.oauth`; the API token continues to live at `tech.reyem.stint.solidtime`. Switching modes does NOT delete the other credential — both can coexist; only the active one is used.

**Tech Stack:** `oauth2` crate (PKCE + authorization code + refresh) · `keyring` (existing, macOS Keychain) · `reqwest` (existing) · `tokio` (existing, async runtime + ephemeral HTTP listener) · `webbrowser` crate (open system browser to authorization URL from CLI; GUI uses the existing `tauri-plugin-opener`).

---

## Why a `TokenProvider` trait (and not an `enum SolidtimeAuth` inside the client)

The spec at §5 sketches a `SolidtimeAuth` enum. In practice this becomes awkward because every API call has to `match` on the variant to figure out how to authenticate. With three providers and ~8 API methods that's 24 match-arms full of similar refresh logic. A trait collapses that to one `bearer_auth(provider.access_token().await?)` call site per method.

The trait also makes testing trivial: tests construct a `SolidtimeClient` with a `MockTokenProvider` that returns a known string. No mocking of the Keychain, no real OAuth dance. Production code wires up `Arc<ApiTokenProvider>` or `Arc<OAuthTokenProvider>` at construction time based on `solidtime.auth_mode`.

The named "SolidtimeAuth" remains in the docs as a conceptual umbrella for "which auth method does the user prefer," but in code it materialises as the choice of which `TokenProvider` to wire up.

## Why store OAuth credentials as one JSON blob (instead of N Keychain entries)

A token refresh atomically rotates `access_token`, `refresh_token`, and `expires_at`. Three separate Keychain writes leave a window where the entries are inconsistent (e.g., refreshed `access_token` paired with stale `refresh_token` if a write fails in between). One JSON blob serialized with `serde_json` to a single Keychain entry serialises the rotation. Storage cost is identical (single Keychain credential).

`client_id` lives in the same blob even though it isn't a secret — it's metadata about the OAuth registration. Keeping it next to the tokens means `OAuthTokenProvider::from_keychain()` is one read, not two.

---

## What ships in Phase 3a (and what does NOT)

**In scope:**
- Shared OAuth PKCE machinery: PKCE code_verifier/code_challenge generation, loopback redirect-capture server, token endpoint exchange, refresh, Keychain persistence helpers.
- `TokenProvider` trait + `ApiTokenProvider` + `OAuthTokenProvider`.
- Refactor of `SolidtimeClient` to use `Arc<dyn TokenProvider>`.
- New `solidtime.auth_mode` settings key.
- CLI command `stint config login` (triggers OAuth flow).
- CLI command `stint config logout` (deletes OAuth blob from Keychain; sets `auth_mode` back to `api_token` if API token present, otherwise leaves it as `oauth` for re-login).
- Tauri commands + Settings UI surface for switching auth methods and signing in.
- README + CLAUDE.md + AGENTS.md updates including the `php artisan` snippet for Solidtime client registration.

**Out of scope (deferred to later sub-phases):**
- Calendar integration (Phase 3b/c/d): no `CalendarProvider` trait, no schema for `calendar_accounts` / `calendar_events`, no calendar UI, no iCal parsing.
- Google Calendar / Microsoft Graph providers.
- CalDAV.
- OAuth client auto-registration / dynamic-client-registration on Solidtime (not supported by Solidtime today anyway).
- Encryption-at-rest of the OAuth blob beyond what Keychain already provides.

---

## File Structure

```
stint/
├── Cargo.toml                                       # MODIFIED — add `oauth2` and `webbrowser` to [workspace.dependencies]
├── crates/
│   ├── stint-core/
│   │   ├── Cargo.toml                               # MODIFIED — depend on oauth2
│   │   └── src/
│   │       ├── lib.rs                               # MODIFIED — add `pub mod oauth;` and re-exports
│   │       ├── error.rs                             # MODIFIED — add OAuth error variants
│   │       ├── oauth/                               # NEW — provider-agnostic OAuth 2.0 machinery
│   │       │   ├── mod.rs                           # public surface: OAuthConfig, OAuthClient, TokenSet
│   │       │   ├── pkce.rs                          # PKCE code_verifier + code_challenge
│   │       │   ├── tokens.rs                        # TokenSet + serde + expiry math
│   │       │   ├── client.rs                        # OAuthClient: authorize URL, code exchange, refresh
│   │       │   └── loopback.rs                      # ephemeral 127.0.0.1 HTTP listener for redirect capture
│   │       └── solidtime/
│   │           ├── mod.rs                           # MODIFIED — SolidtimeClient holds Arc<dyn TokenProvider>
│   │           ├── dto.rs                           # unchanged
│   │           └── auth.rs                          # NEW — TokenProvider trait + ApiTokenProvider + OAuthTokenProvider
│   ├── stint-cli/
│   │   ├── Cargo.toml                               # MODIFIED — depend on webbrowser
│   │   └── src/
│   │       ├── main.rs                              # MODIFIED — register `login` and `logout` subcommands of `config`
│   │       └── config_login.rs                      # NEW — interactive OAuth flow handler
│   └── stint-app/
│       └── src/
│           └── commands/
│               └── config.rs                        # MODIFIED — add oauth_solidtime_start/status/logout commands
└── ui/
    └── src/
        └── routes/
            └── Settings.tsx                         # MODIFIED — auth method radio + Sign in button + status pill
```

After Phase 3a lands, a user can:

- Run `stint config login` in the terminal → browser opens, they approve, terminal prints "Signed in as <email>".
- Switch back to API token by re-pasting it in Settings (or via `stint config set solidtime.token`).
- Open the GUI Settings panel → pick **API token** or **OAuth** → if OAuth, click **Sign in with Solidtime**.
- Use the app normally; OAuth access-tokens refresh transparently when they expire.

---

## Cross-task setup

- **Working directory:** `/Users/mariomeyer/code/ReyemTech/apps/tet`
- **Branch:** `phase-3a`, branched from `main`.
- **Commits:** Conventional Commits as before. Prefixes used here: `feat(core)`, `feat(cli)`, `feat(app)`, `feat(ui)`, `test(core)`, `refactor(core)`, `chore(deps)`, `docs`, `fix(*)`.
- **End-state check after each task:** `cargo check --workspace` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean; tests for the task pass under `STINT_SKIP_KEYCHAIN_TESTS=1 cargo test --workspace -- --test-threads=1`. (Local devs can also run without the env var; CI will run with it.)
- **TDD discipline:** every task that adds source code in `stint-core` follows write-failing-test → confirm-fail → minimal-impl → confirm-pass → commit. The OAuth code is security-sensitive; tests are the contract.
- **PR strategy:** open a draft PR after Task 1 (so CI exercises every later commit). Mark ready after Task 18. Merge via "Rebase and merge" per the workflow set up in Phase 2.5. Tag `phase-3a-complete` on the resulting `main`.
- **Push policy:** branch protection on `main` is active. All commits land via PR; no direct pushes to `main` allowed.

---

## Tasks

### Task 1: Branch, add `oauth2` + `webbrowser` dependencies, scaffold module

**Files:**
- Modify: `Cargo.toml` (root workspace `[workspace.dependencies]`)
- Modify: `crates/stint-core/Cargo.toml`
- Modify: `crates/stint-cli/Cargo.toml`
- Create: `crates/stint-core/src/oauth/mod.rs`
- Create: `crates/stint-core/src/oauth/pkce.rs`
- Create: `crates/stint-core/src/oauth/tokens.rs`
- Create: `crates/stint-core/src/oauth/client.rs`
- Create: `crates/stint-core/src/oauth/loopback.rs`
- Create: `crates/stint-core/src/solidtime/auth.rs`
- Modify: `crates/stint-core/src/lib.rs`
- Modify: `crates/stint-core/src/solidtime/mod.rs`

- [ ] **Step 1: Confirm clean tree on `main` and branch**

```bash
git status        # must be clean
git checkout -b phase-3a
```

- [ ] **Step 2: Edit root `Cargo.toml` to add OAuth + webbrowser deps**

In `[workspace.dependencies]`, after the `# http` section, add:

```toml
# oauth
oauth2 = { version = "5", default-features = false, features = ["reqwest", "rustls-tls"] }
webbrowser = "1"
```

NOTE: `oauth2` v5 uses `reqwest` as the HTTP transport. Disabling default features avoids pulling native-tls; we use rustls-tls to match the rest of the workspace.

- [ ] **Step 3: Edit `crates/stint-core/Cargo.toml` to depend on `oauth2`**

In the `[dependencies]` section, after the `keyring` line, add:

```toml
oauth2.workspace = true
```

- [ ] **Step 4: Edit `crates/stint-cli/Cargo.toml` to depend on `webbrowser`**

In `[dependencies]`, after the existing entries, add:

```toml
webbrowser.workspace = true
```

- [ ] **Step 5: Create stub module files (each one-line `// stub`)**

```bash
mkdir -p crates/stint-core/src/oauth
```

Then create these files with a single `// stub` line each:
- `crates/stint-core/src/oauth/mod.rs`
- `crates/stint-core/src/oauth/pkce.rs`
- `crates/stint-core/src/oauth/tokens.rs`
- `crates/stint-core/src/oauth/client.rs`
- `crates/stint-core/src/oauth/loopback.rs`
- `crates/stint-core/src/solidtime/auth.rs`

- [ ] **Step 6: Wire them into the module tree**

Edit `crates/stint-core/src/oauth/mod.rs`:

```rust
//! Provider-agnostic OAuth 2.0 (PKCE + authorization code) machinery.
//!
//! Used by `stint-core::solidtime::auth::OAuthTokenProvider` and (in
//! future phases) by calendar providers.

pub mod client;
pub mod loopback;
pub mod pkce;
pub mod tokens;
```

Edit `crates/stint-core/src/solidtime/mod.rs` — at the top, add:

```rust
pub mod auth;
```

Edit `crates/stint-core/src/lib.rs` — find the `pub mod` declarations and add (alphabetical order):

```rust
pub mod oauth;
```

- [ ] **Step 7: Verify the workspace still builds**

```bash
cargo check --workspace
```

Expected: clean compile.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml crates/stint-core/Cargo.toml crates/stint-cli/Cargo.toml \
        crates/stint-core/src/oauth crates/stint-core/src/solidtime/auth.rs \
        crates/stint-core/src/lib.rs crates/stint-core/src/solidtime/mod.rs
git commit -m "chore(deps): scaffold oauth module and add oauth2 + webbrowser

Adds oauth2 v5 (with rustls + reqwest features) and webbrowser to
the workspace. Creates empty modules under stint-core::oauth and
stint-core::solidtime::auth; subsequent tasks fill them in. No
behaviour change."
```

---

### Task 2: PKCE primitives — `code_verifier` and `code_challenge`

**Files:**
- Modify: `crates/stint-core/src/oauth/pkce.rs`
- Create: `crates/stint-core/tests/oauth_pkce.rs`

PKCE (RFC 7636): generate a high-entropy random `code_verifier` (43–128 chars from [A-Z][a-z][0-9]-._~), then `code_challenge = BASE64URL(SHA256(code_verifier))`.

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/oauth_pkce.rs`:

```rust
use stint_core::oauth::pkce::{code_challenge_for, generate_verifier};

#[test]
fn verifier_is_43_to_128_chars_of_allowed_alphabet() {
    let v = generate_verifier();
    let len = v.len();
    assert!((43..=128).contains(&len), "verifier length {len} out of range");
    assert!(
        v.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~')),
        "verifier contains disallowed char: {v}"
    );
}

#[test]
fn two_verifiers_in_a_row_are_distinct() {
    let a = generate_verifier();
    let b = generate_verifier();
    assert_ne!(a, b, "PRNG produced two identical verifiers — high entropy lost?");
}

#[test]
fn challenge_is_base64url_sha256_of_verifier() {
    // Known test vector from RFC 7636 §4.4.
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let expected_challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
    assert_eq!(code_challenge_for(verifier), expected_challenge);
}
```

- [ ] **Step 2: Run test — expect compile failure (functions don't exist)**

```bash
cargo test -p stint-core --test oauth_pkce 2>&1 | head -20
```

Expected: errors about `generate_verifier`/`code_challenge_for` not found.

- [ ] **Step 3: Implement `pkce.rs`**

Replace `crates/stint-core/src/oauth/pkce.rs` content with:

```rust
//! PKCE (RFC 7636) primitives: code_verifier and code_challenge.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};

const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";

/// RFC 7636 §4.1: a random string of 43..=128 chars from the unreserved set.
/// We pick a fixed 64 — comfortably above the 43 minimum, well under 128.
pub fn generate_verifier() -> String {
    let mut bytes = [0u8; 64];
    OsRng.fill_bytes(&mut bytes);
    bytes
        .iter()
        .map(|b| ALPHABET[(*b as usize) % ALPHABET.len()] as char)
        .collect()
}

/// RFC 7636 §4.2: `BASE64URL(SHA256(verifier))`, no padding.
pub fn code_challenge_for(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}
```

The `oauth2` crate transitively provides `base64`, `rand`, and `sha2` via its dependency set, BUT we depend on them directly to avoid coupling. Add them to `crates/stint-core/Cargo.toml` under `[dependencies]`:

```toml
base64 = "0.22"
rand = "0.8"
sha2 = "0.10"
```

(These crates are stable and small. If your `oauth2` v5 install pulls them transitively, cargo will dedupe versions in the lockfile.)

- [ ] **Step 4: Run test — expect PASS**

```bash
cargo test -p stint-core --test oauth_pkce -- --nocapture
```

Expected: 3 tests pass.

- [ ] **Step 5: Run `cargo clippy --workspace --all-targets -- -D warnings`**

Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add crates/stint-core/src/oauth/pkce.rs crates/stint-core/tests/oauth_pkce.rs \
        crates/stint-core/Cargo.toml
git commit -m "feat(core): PKCE code_verifier and code_challenge generation

Implements RFC 7636 §4.1 (verifier — 64 chars from the unreserved set,
sampled via OsRng) and §4.2 (challenge — BASE64URL-SHA256 of verifier).
Verified against the RFC 7636 §4.4 known test vector."
```

---

### Task 3: `TokenSet` — typed token bundle with expiry math

**Files:**
- Modify: `crates/stint-core/src/oauth/tokens.rs`
- Create: `crates/stint-core/tests/oauth_tokens.rs`

A `TokenSet` is what an OAuth provider hands back after a successful exchange or refresh: `access_token`, `refresh_token` (optional in some grants, present in our case), `expires_at` absolute timestamp, and (optionally) a `token_type` and `scope` echo.

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/oauth_tokens.rs`:

```rust
use chrono::{Duration, Utc};
use stint_core::oauth::tokens::TokenSet;

#[test]
fn from_response_computes_expires_at_from_expires_in() {
    let now = Utc::now();
    let t = TokenSet::from_response(
        "access-1".into(),
        Some("refresh-1".into()),
        3600, // expires_in seconds
        Some("read".into()),
        now,
    );
    assert_eq!(t.access_token, "access-1");
    assert_eq!(t.refresh_token.as_deref(), Some("refresh-1"));
    assert!(
        (t.expires_at - now - Duration::seconds(3600)).num_milliseconds().abs() < 10,
        "expires_at should be now + 3600s"
    );
    assert_eq!(t.scope.as_deref(), Some("read"));
}

#[test]
fn is_expired_with_skew_is_true_inside_safety_window() {
    let now = Utc::now();
    let t = TokenSet::from_response("a".into(), Some("r".into()), 30, None, now);
    // 30s expiry, default skew is 60s, so it's already "expired" for safety.
    assert!(t.is_expired_with_skew(now), "should be expired due to skew");
}

#[test]
fn is_expired_with_skew_is_false_when_plenty_of_time_left() {
    let now = Utc::now();
    let t = TokenSet::from_response("a".into(), Some("r".into()), 3600, None, now);
    assert!(!t.is_expired_with_skew(now));
}

#[test]
fn refresh_preserves_refresh_token_when_response_omits_it() {
    // Some providers (e.g., Solidtime/Passport) include refresh_token in every
    // response; others (e.g., Google) only on initial issue. If a refresh
    // response omits it, we MUST keep the existing one.
    let original = TokenSet::from_response("a1".into(), Some("r1".into()), 60, None, Utc::now());
    let merged = original.merge_refresh_response("a2".into(), None, 120, None);
    assert_eq!(merged.access_token, "a2");
    assert_eq!(merged.refresh_token.as_deref(), Some("r1"));
}

#[test]
fn refresh_overwrites_refresh_token_when_response_includes_one() {
    let original = TokenSet::from_response("a1".into(), Some("r1".into()), 60, None, Utc::now());
    let merged = original.merge_refresh_response("a2".into(), Some("r2".into()), 120, None);
    assert_eq!(merged.access_token, "a2");
    assert_eq!(merged.refresh_token.as_deref(), Some("r2"));
}
```

- [ ] **Step 2: Run test — expect compile failure**

```bash
cargo test -p stint-core --test oauth_tokens 2>&1 | head -20
```

Expected: `TokenSet` not found.

- [ ] **Step 3: Implement `tokens.rs`**

Replace `crates/stint-core/src/oauth/tokens.rs` content with:

```rust
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
```

- [ ] **Step 4: Run test — expect PASS**

```bash
cargo test -p stint-core --test oauth_tokens
```

Expected: 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/oauth/tokens.rs crates/stint-core/tests/oauth_tokens.rs
git commit -m "feat(core): TokenSet with expiry-skew and refresh-merge semantics

TokenSet bundles access/refresh tokens and absolute expiry.
- from_response: compute expires_at from expires_in seconds.
- is_expired_with_skew: 60s safety margin so we refresh before the
  wire deadline.
- merge_refresh_response: preserves an existing refresh_token if the
  refresh response omitted it (Solidtime always returns one; some
  providers don't)."
```

---

### Task 4: Error variants for OAuth

**Files:**
- Modify: `crates/stint-core/src/error.rs`

The OAuth code paths need their own error variants for actionable handling (auth failed, refresh expired, redirect-server bind failed, etc.).

- [ ] **Step 1: Read current `error.rs`**

```bash
cat crates/stint-core/src/error.rs
```

Note the existing variants. The new ones go at the end of the `enum Error` definition before the closing brace.

- [ ] **Step 2: Add OAuth variants**

Add to the `enum Error` definition (use Edit to insert before the closing `}` of the enum):

```rust
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
```

The exact placement: between the last existing variant and the closing `}`. Use `cargo check -p stint-core` to confirm the addition compiles before committing.

- [ ] **Step 3: Verify compile**

```bash
cargo check -p stint-core
```

Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add crates/stint-core/src/error.rs
git commit -m "feat(core): add OAuth error variants

Adds OAuthCancelled, OAuthServer, OAuthRefreshFailed, OAuthStateMismatch,
and OAuthLoopback for actionable error handling by the OAuth machinery
landing in subsequent tasks."
```

---

### Task 5: `OAuthConfig` and authorize-URL builder

**Files:**
- Modify: `crates/stint-core/src/oauth/client.rs`
- Create: `crates/stint-core/tests/oauth_authorize_url.rs`

`OAuthConfig` is a small struct holding everything an `OAuthClient` needs: authorization endpoint, token endpoint, client_id, redirect_uri, requested scopes. The authorize-URL builder constructs the standard `?response_type=code&client_id=...&...&code_challenge=...&code_challenge_method=S256` URL and returns the `code_verifier` so the caller can later use it in the token exchange.

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/oauth_authorize_url.rs`:

```rust
use stint_core::oauth::client::{OAuthClient, OAuthConfig};

fn cfg() -> OAuthConfig {
    OAuthConfig {
        authorize_url: "https://time.example.com/oauth/authorize".into(),
        token_url: "https://time.example.com/oauth/token".into(),
        client_id: "stint-desktop".into(),
        redirect_uri: "http://127.0.0.1:54321/callback".into(),
        scopes: vec!["read".into(), "create".into(), "update".into(), "delete".into()],
    }
}

#[test]
fn authorize_url_includes_pkce_and_state() {
    let client = OAuthClient::new(cfg());
    let prepared = client.prepare_authorize();
    let url = prepared.authorize_url.as_str();
    assert!(url.starts_with("https://time.example.com/oauth/authorize?"));
    assert!(url.contains("response_type=code"));
    assert!(url.contains("client_id=stint-desktop"));
    assert!(url.contains("redirect_uri=http"));
    assert!(url.contains("scope=read"));
    assert!(url.contains("code_challenge_method=S256"));
    assert!(url.contains("code_challenge="));
    assert!(url.contains("state="));
    assert!(!prepared.code_verifier.is_empty());
    assert!(!prepared.state.is_empty());
}

#[test]
fn two_prepares_produce_distinct_verifiers_and_states() {
    let client = OAuthClient::new(cfg());
    let a = client.prepare_authorize();
    let b = client.prepare_authorize();
    assert_ne!(a.code_verifier, b.code_verifier);
    assert_ne!(a.state, b.state);
}
```

- [ ] **Step 2: Run test — expect compile failure**

```bash
cargo test -p stint-core --test oauth_authorize_url 2>&1 | head -15
```

Expected: types not found.

- [ ] **Step 3: Implement `client.rs` (authorize-URL portion)**

Replace `crates/stint-core/src/oauth/client.rs` content with:

```rust
//! OAuthClient — provider-agnostic PKCE flow driver.
//!
//! `OAuthClient` does NOT itself open a browser or run the redirect server;
//! those concerns live in `crate::oauth::loopback` and the calling surface
//! (CLI / Tauri command). This module is testable in isolation.

use crate::oauth::pkce;
use rand::rngs::OsRng;
use rand::RngCore;
use url::Url;

#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub authorize_url: String,
    pub token_url: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
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

        PreparedAuthorize {
            authorize_url: url,
            code_verifier,
            state,
        }
    }
}

fn random_state() -> String {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}
```

Add `url` to `crates/stint-core/Cargo.toml` under `[dependencies]`:

```toml
url = "2"
```

Note: we deliberately do NOT use the `oauth2` crate's `BasicClient` for URL construction. The crate's API is awkward for our needs (it requires `oauth2::url::Url` types throughout, owns the redirect-URI parsing, and its `authorize_url()` returns a tuple with PKCE state that's harder to compose with our state-machine). We use `oauth2` for the token-endpoint POST in Tasks 7/8 only, where its handling of typed responses is genuinely useful.

- [ ] **Step 4: Run test — expect PASS**

```bash
cargo test -p stint-core --test oauth_authorize_url
```

Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/oauth/client.rs crates/stint-core/tests/oauth_authorize_url.rs \
        crates/stint-core/Cargo.toml
git commit -m "feat(core): OAuthConfig + OAuthClient with authorize-URL builder

prepare_authorize() returns the authorize URL, a fresh PKCE code_verifier,
and a CSRF state — caller is responsible for opening a browser at the
URL and holding the verifier/state until the redirect comes back."
```

---

### Task 6: Loopback redirect-capture server

**Files:**
- Modify: `crates/stint-core/src/oauth/loopback.rs`
- Create: `crates/stint-core/tests/oauth_loopback.rs`

The loopback server binds `127.0.0.1:0` (kernel picks a free port), serves exactly one GET request on `/callback?code=...&state=...`, replies with a small HTML page telling the user to close the tab, then shuts down. The caller gets back the captured `(code, state)` plus the actual bound port so they can include it in the redirect_uri.

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/oauth_loopback.rs`:

```rust
use std::time::Duration;
use stint_core::oauth::loopback::{listen_for_callback, CapturedCallback};

#[tokio::test]
async fn captures_code_and_state_from_callback() {
    let server = listen_for_callback(Duration::from_secs(5))
        .await
        .expect("bind loopback");
    let port = server.port();
    let url = format!("http://127.0.0.1:{port}/callback?code=test-code&state=test-state");

    // In a real flow the browser hits this URL. We simulate it with reqwest.
    tokio::spawn(async move {
        let _ = reqwest::get(&url).await;
    });

    let captured: CapturedCallback = server.await_callback().await.expect("capture");
    assert_eq!(captured.code, "test-code");
    assert_eq!(captured.state, "test-state");
}

#[tokio::test]
async fn returns_oauth_server_error_when_callback_carries_error_param() {
    let server = listen_for_callback(Duration::from_secs(5))
        .await
        .expect("bind loopback");
    let port = server.port();
    let url = format!(
        "http://127.0.0.1:{port}/callback?error=access_denied&error_description=User+rejected"
    );
    tokio::spawn(async move {
        let _ = reqwest::get(&url).await;
    });
    let err = server.await_callback().await.unwrap_err();
    match err {
        stint_core::Error::OAuthServer(msg) => {
            assert!(msg.contains("access_denied"), "got: {msg}");
        }
        e => panic!("expected OAuthServer, got {e:?}"),
    }
}

#[tokio::test]
async fn times_out_with_oauth_cancelled() {
    let server = listen_for_callback(Duration::from_millis(200))
        .await
        .expect("bind loopback");
    // Don't hit the callback URL — let it time out.
    let err = server.await_callback().await.unwrap_err();
    assert!(matches!(err, stint_core::Error::OAuthCancelled), "got {err:?}");
}
```

- [ ] **Step 2: Run test — expect compile failure**

```bash
cargo test -p stint-core --test oauth_loopback 2>&1 | head -20
```

Expected: types not found.

- [ ] **Step 3: Implement `loopback.rs`**

Replace `crates/stint-core/src/oauth/loopback.rs` content with:

```rust
//! Ephemeral 127.0.0.1 HTTP listener for OAuth redirect capture.

use crate::{Error, Result};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::time::timeout;

const SUCCESS_HTML: &str =
    "<!doctype html><meta charset=utf-8><title>stint — signed in</title>\
<style>body{font:16px system-ui;padding:48px;max-width:520px;color:#1a1a1a}</style>\
<h1>Signed in to Solidtime</h1>\
<p>You can close this tab and return to stint.</p>";

const ERROR_HTML: &str =
    "<!doctype html><meta charset=utf-8><title>stint — sign-in failed</title>\
<style>body{font:16px system-ui;padding:48px;max-width:520px;color:#1a1a1a}</style>\
<h1>Sign-in failed</h1>\
<p>Return to stint for details.</p>";

pub struct CapturedCallback {
    pub code: String,
    pub state: String,
}

pub struct LoopbackServer {
    port: u16,
    rx: oneshot::Receiver<Result<CapturedCallback>>,
    timeout: Duration,
}

impl LoopbackServer {
    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn await_callback(self) -> Result<CapturedCallback> {
        match timeout(self.timeout, self.rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(Error::OAuthCancelled),
            Err(_) => Err(Error::OAuthCancelled),
        }
    }
}

pub async fn listen_for_callback(server_timeout: Duration) -> Result<LoopbackServer> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| Error::OAuthLoopback(e.to_string()))?;
    let port = listener
        .local_addr()
        .map_err(|e| Error::OAuthLoopback(e.to_string()))?
        .port();
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        let Ok((mut socket, _)) = listener.accept().await else {
            let _ = tx.send(Err(Error::OAuthCancelled));
            return;
        };

        // Read the request line: "GET /callback?... HTTP/1.1\r\n..."
        let mut reader = BufReader::new(&mut socket);
        let mut request_line = String::new();
        if reader.read_line(&mut request_line).await.is_err() {
            let _ = tx.send(Err(Error::OAuthCancelled));
            return;
        }

        let parse_result = parse_callback_query(&request_line);
        let (body, response) = match &parse_result {
            Ok(_) => (SUCCESS_HTML, "HTTP/1.1 200 OK"),
            Err(_) => (ERROR_HTML, "HTTP/1.1 400 Bad Request"),
        };

        let payload = format!(
            "{response}\r\nContent-Type: text/html; charset=utf-8\r\n\
Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = socket.write_all(payload.as_bytes()).await;
        let _ = socket.shutdown().await;

        let _ = tx.send(parse_result);
    });

    Ok(LoopbackServer {
        port,
        rx,
        timeout: server_timeout,
    })
}

/// Parse "GET /callback?code=...&state=... HTTP/1.1\r\n" → CapturedCallback.
fn parse_callback_query(request_line: &str) -> Result<CapturedCallback> {
    let path_and_query = request_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| Error::OAuthServer("malformed HTTP request line".into()))?;
    let query = path_and_query
        .split_once('?')
        .map(|(_, q)| q)
        .unwrap_or_default();

    let mut code = None;
    let mut state = None;
    let mut error = None;
    let mut error_desc = None;
    for pair in query.split('&') {
        let Some((k, v)) = pair.split_once('=') else { continue };
        let v = percent_decode(v);
        match k {
            "code" => code = Some(v),
            "state" => state = Some(v),
            "error" => error = Some(v),
            "error_description" => error_desc = Some(v),
            _ => {}
        }
    }

    if let Some(e) = error {
        let msg = match error_desc {
            Some(d) => format!("{e}: {d}"),
            None => e,
        };
        return Err(Error::OAuthServer(msg));
    }

    let code = code.ok_or_else(|| Error::OAuthServer("callback missing code".into()))?;
    let state = state.ok_or_else(|| Error::OAuthServer("callback missing state".into()))?;
    Ok(CapturedCallback { code, state })
}

fn percent_decode(s: &str) -> String {
    // Minimal `+` and `%XX` decode — sufficient for OAuth params.
    let mut out = String::with_capacity(s.len());
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        match b {
            b'+' => out.push(' '),
            b'%' => {
                let h1 = bytes.next();
                let h2 = bytes.next();
                if let (Some(h1), Some(h2)) = (h1, h2) {
                    if let (Some(d1), Some(d2)) =
                        ((h1 as char).to_digit(16), (h2 as char).to_digit(16))
                    {
                        out.push(((d1 * 16 + d2) as u8) as char);
                    }
                }
            }
            _ => out.push(b as char),
        }
    }
    out
}
```

- [ ] **Step 4: Run test — expect PASS**

```bash
cargo test -p stint-core --test oauth_loopback -- --test-threads=1
```

Expected: 3 tests pass. (Single-threaded — multiple async tests binding to 127.0.0.1 are isolated by random ports, but the timeout test is timing-sensitive so we run serial for reliability.)

- [ ] **Step 5: Run clippy**

```bash
cargo clippy -p stint-core --all-targets -- -D warnings
```

Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add crates/stint-core/src/oauth/loopback.rs crates/stint-core/tests/oauth_loopback.rs
git commit -m "feat(core): ephemeral loopback HTTP server for OAuth redirect capture

Binds 127.0.0.1:<random-port>, accepts one request, returns a small
HTML 'you can close this tab' page, and resolves the captured
(code, state) pair to the caller. Surfaces OAuthServer for upstream
?error=, OAuthCancelled for timeouts, OAuthLoopback for bind failures."
```

---

### Task 7: Token exchange (authorization_code grant)

**Files:**
- Modify: `crates/stint-core/src/oauth/client.rs` (add `exchange_code` method)
- Create: `crates/stint-core/tests/oauth_exchange.rs`

Given `(code, code_verifier)`, POST to the token endpoint and return a `TokenSet`. Wiremock-driven test mirrors the Phase 1 / Phase 2 wiremock pattern in `tests/solidtime.rs`.

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/oauth_exchange.rs`:

```rust
use stint_core::oauth::client::{OAuthClient, OAuthConfig};
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn cfg(base: &str) -> OAuthConfig {
    OAuthConfig {
        authorize_url: format!("{base}/oauth/authorize"),
        token_url: format!("{base}/oauth/token"),
        client_id: "stint-desktop".into(),
        redirect_uri: "http://127.0.0.1:54321/callback".into(),
        scopes: vec!["read".into(), "create".into()],
    }
}

#[tokio::test]
async fn exchange_code_posts_form_and_parses_tokens() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=authorization_code"))
        .and(body_string_contains("code=test-code"))
        .and(body_string_contains("code_verifier=test-verifier"))
        .and(body_string_contains("client_id=stint-desktop"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "token_type": "Bearer",
            "expires_in": 3600,
            "access_token": "access-1",
            "refresh_token": "refresh-1",
            "scope": "read create"
        })))
        .mount(&server)
        .await;

    let client = OAuthClient::new(cfg(&server.uri()));
    let tokens = client
        .exchange_code("test-code", "test-verifier")
        .await
        .unwrap();
    assert_eq!(tokens.access_token, "access-1");
    assert_eq!(tokens.refresh_token.as_deref(), Some("refresh-1"));
    assert_eq!(tokens.scope.as_deref(), Some("read create"));
}

#[tokio::test]
async fn exchange_code_surfaces_oauth_server_on_4xx() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": "invalid_grant",
            "error_description": "Authorization code has expired"
        })))
        .mount(&server)
        .await;

    let client = OAuthClient::new(cfg(&server.uri()));
    let err = client.exchange_code("test-code", "test-verifier").await.unwrap_err();
    match err {
        stint_core::Error::OAuthServer(msg) => {
            assert!(msg.contains("invalid_grant"), "got {msg}");
            assert!(msg.contains("expired"), "got {msg}");
        }
        e => panic!("expected OAuthServer, got {e:?}"),
    }
}
```

- [ ] **Step 2: Run test — expect compile failure (method missing)**

```bash
cargo test -p stint-core --test oauth_exchange 2>&1 | head -15
```

Expected: `exchange_code` not found on `OAuthClient`.

- [ ] **Step 3: Implement `exchange_code` in `client.rs`**

Add to the top of `crates/stint-core/src/oauth/client.rs`:

```rust
use crate::oauth::tokens::TokenSet;
use crate::{Error, Result};
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
```

(Adjust the existing imports — `Result`/`Error` etc. — to avoid duplication.)

Add a method to `impl OAuthClient`:

```rust
    pub async fn exchange_code(&self, code: &str, code_verifier: &str) -> Result<TokenSet> {
        let http = Client::new();
        let form = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &self.config.redirect_uri),
            ("client_id", &self.config.client_id),
            ("code_verifier", code_verifier),
        ];
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
```

Add this struct at the bottom of the file:

```rust
#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
    scope: Option<String>,
}
```

- [ ] **Step 4: Run test — expect PASS**

```bash
cargo test -p stint-core --test oauth_exchange
```

Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/oauth/client.rs crates/stint-core/tests/oauth_exchange.rs
git commit -m "feat(core): OAuthClient::exchange_code (authorization_code grant)

POSTs application/x-www-form-urlencoded body to the token endpoint with
grant_type=authorization_code + code + code_verifier + client_id +
redirect_uri. Parses the standard JSON response into a TokenSet.
Surfaces OAuthServer with HTTP status + body for non-2xx responses."
```

---

### Task 8: Token refresh (refresh_token grant)

**Files:**
- Modify: `crates/stint-core/src/oauth/client.rs` (add `refresh_tokens` method)
- Create: `crates/stint-core/tests/oauth_refresh.rs`

Mirrors `exchange_code` but with `grant_type=refresh_token` and the refresh-token as the credential. Returns a fresh `TokenSet`.

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/oauth_refresh.rs`:

```rust
use stint_core::oauth::client::{OAuthClient, OAuthConfig};
use stint_core::oauth::tokens::TokenSet;
use chrono::Utc;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn cfg(base: &str) -> OAuthConfig {
    OAuthConfig {
        authorize_url: format!("{base}/oauth/authorize"),
        token_url: format!("{base}/oauth/token"),
        client_id: "stint-desktop".into(),
        redirect_uri: "http://127.0.0.1:54321/callback".into(),
        scopes: vec!["read".into()],
    }
}

#[tokio::test]
async fn refresh_posts_form_and_returns_new_token_set() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=refresh_token"))
        .and(body_string_contains("refresh_token=old-refresh"))
        .and(body_string_contains("client_id=stint-desktop"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "token_type": "Bearer",
            "expires_in": 3600,
            "access_token": "access-2",
            "refresh_token": "new-refresh",
            "scope": "read"
        })))
        .mount(&server)
        .await;

    let client = OAuthClient::new(cfg(&server.uri()));
    let prior = TokenSet::from_response("access-1".into(), Some("old-refresh".into()), 60, None, Utc::now());
    let refreshed = client.refresh_tokens(&prior).await.unwrap();
    assert_eq!(refreshed.access_token, "access-2");
    assert_eq!(refreshed.refresh_token.as_deref(), Some("new-refresh"));
}

#[tokio::test]
async fn refresh_returns_oauth_refresh_failed_on_invalid_grant() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": "invalid_grant",
            "error_description": "Refresh token expired"
        })))
        .mount(&server)
        .await;

    let client = OAuthClient::new(cfg(&server.uri()));
    let prior = TokenSet::from_response("a".into(), Some("expired-r".into()), 0, None, Utc::now());
    let err = client.refresh_tokens(&prior).await.unwrap_err();
    assert!(matches!(err, stint_core::Error::OAuthRefreshFailed), "got {err:?}");
}
```

- [ ] **Step 2: Run test — expect failure**

```bash
cargo test -p stint-core --test oauth_refresh 2>&1 | head -10
```

- [ ] **Step 3: Implement `refresh_tokens` in `client.rs`**

Add to `impl OAuthClient`:

```rust
    pub async fn refresh_tokens(&self, prior: &TokenSet) -> Result<TokenSet> {
        let refresh_token = prior
            .refresh_token
            .as_deref()
            .ok_or(Error::OAuthRefreshFailed)?;

        let http = Client::new();
        let form = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.config.client_id),
        ];
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
        let parsed: TokenResponse = resp
            .json()
            .await
            .map_err(|_| Error::OAuthRefreshFailed)?;
        Ok(prior.merge_refresh_response(
            parsed.access_token,
            parsed.refresh_token,
            parsed.expires_in,
            parsed.scope,
        ))
    }
```

- [ ] **Step 4: Run test — expect PASS**

```bash
cargo test -p stint-core --test oauth_refresh
```

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/oauth/client.rs crates/stint-core/tests/oauth_refresh.rs
git commit -m "feat(core): OAuthClient::refresh_tokens (refresh_token grant)

POSTs grant_type=refresh_token with the stored refresh-token. Maps
4xx responses to Error::OAuthRefreshFailed (the user must re-auth).
Preserves the existing refresh-token when the response omits one
via TokenSet::merge_refresh_response."
```

---

### Task 9: `TokenProvider` trait + `ApiTokenProvider`

**Files:**
- Modify: `crates/stint-core/src/solidtime/auth.rs`
- Create: `crates/stint-core/tests/solidtime_token_provider.rs`

`TokenProvider` is the abstraction that lets `SolidtimeClient` not care whether it's getting an API token (static) or an OAuth access token (refreshed on demand). `ApiTokenProvider` lands in this task; `OAuthTokenProvider` lands in Task 11.

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/solidtime_token_provider.rs`:

```rust
use std::sync::Arc;
use stint_core::solidtime::auth::{ApiTokenProvider, TokenProvider};

#[tokio::test]
async fn api_token_provider_returns_configured_token() {
    let p: Arc<dyn TokenProvider> = Arc::new(ApiTokenProvider::new("static-token-1".into()));
    let t1 = p.access_token().await.unwrap();
    let t2 = p.access_token().await.unwrap();
    assert_eq!(t1, "static-token-1");
    assert_eq!(t2, "static-token-1");
}
```

- [ ] **Step 2: Implement the trait + impl**

Replace `crates/stint-core/src/solidtime/auth.rs` content with:

```rust
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
```

- [ ] **Step 3: Run test — expect PASS**

```bash
cargo test -p stint-core --test solidtime_token_provider
```

- [ ] **Step 4: Commit**

```bash
git add crates/stint-core/src/solidtime/auth.rs \
        crates/stint-core/tests/solidtime_token_provider.rs
git commit -m "feat(core): TokenProvider trait + ApiTokenProvider

Defines the async trait that SolidtimeClient will depend on instead of
holding a token directly. ApiTokenProvider wraps a static PAT for the
existing auth_mode=api_token path."
```

---

### Task 10: Refactor `SolidtimeClient` to use `Arc<dyn TokenProvider>`

**Files:**
- Modify: `crates/stint-core/src/solidtime/mod.rs`
- Modify: `crates/stint-core/tests/solidtime.rs` (existing tests)
- Modify: `crates/stint-core/tests/sync_push.rs` (existing tests)
- Modify: `crates/stint-core/src/sync/push.rs` (if it constructs the client)
- Modify: `crates/stint-app/src/commands/*.rs` (anywhere the GUI constructs the client)
- Modify: `crates/stint-cli/src/**.rs` (anywhere the CLI constructs the client)

The semantic change: `SolidtimeClient::new(base_url, token)` becomes `SolidtimeClient::new(base_url, token_provider)`. To keep call-site churn minimal, we add `SolidtimeClient::with_api_token(base_url, token)` as a convenience that internally wraps `ApiTokenProvider`. Existing call sites can switch to the convenience constructor with a one-word rename.

- [ ] **Step 1: Modify `SolidtimeClient` to hold `Arc<dyn TokenProvider>`**

Replace `crates/stint-core/src/solidtime/mod.rs`'s `SolidtimeClient` struct + constructors with:

```rust
pub mod auth;
pub mod dto;

use crate::solidtime::auth::{ApiTokenProvider, TokenProvider};
use crate::{Error, Result};
use dto::*;
use reqwest::{Client, RequestBuilder, StatusCode};
use std::sync::Arc;

pub struct SolidtimeClient {
    base_url: String,
    tokens: Arc<dyn TokenProvider>,
    http: Client,
    org_id: Option<String>,
}

impl SolidtimeClient {
    pub fn new(base_url: &str, tokens: Arc<dyn TokenProvider>) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            tokens,
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("reqwest client builds"),
            org_id: None,
        }
    }

    /// Convenience constructor for the common case of a static API token.
    /// Keeps call sites short and preserves the old behaviour.
    pub fn with_api_token(base_url: &str, token: &str) -> Self {
        Self::new(base_url, Arc::new(ApiTokenProvider::new(token.to_string())))
    }

    pub fn with_org(mut self, org_id: impl Into<String>) -> Self {
        self.org_id = Some(org_id.into());
        self
    }

    pub(crate) fn org(&self) -> Result<&str> {
        self.org_id
            .as_deref()
            .ok_or(Error::MissingConfig("solidtime.org_id"))
    }

    async fn authed(&self, builder: RequestBuilder) -> Result<RequestBuilder> {
        let token = self.tokens.access_token().await?;
        Ok(builder.bearer_auth(token))
    }
```

Then update every API method to call `self.authed(...)` instead of `.bearer_auth(&self.token)`. Example refactor for `test_connection`:

```rust
    pub async fn test_connection(&self) -> Result<UserMe> {
        let url = format!("{}/api/v1/users/me", self.base_url);
        let resp = self.authed(self.http.get(&url)).await?.send().await?;
        let status = resp.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err(Error::SolidtimeAuth);
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::Solidtime { status: status.as_u16(), body });
        }
        let wrapper: Wrapper<UserMe> = resp.json().await?;
        Ok(wrapper.data)
    }
```

Apply the same shape to `list_projects`, `list_tasks`, `list_tags`, `list_memberships`, `get_list` (helper — change signature to take a closure or simply inline the auth call before each `get` if simpler), `create_time_entry`, `update_time_entry`, `delete_time_entry`. Use Edit per method; do not rewrite the whole file.

- [ ] **Step 2: Update call sites in tests**

In `crates/stint-core/tests/solidtime.rs`, replace every `SolidtimeClient::new(&server.uri(), "t")` with `SolidtimeClient::with_api_token(&server.uri(), "t")`. Same for any other `SolidtimeClient::new(...)` calls. Use `git grep -n 'SolidtimeClient::new'` to find them all.

In `crates/stint-core/tests/sync_push.rs`, same change.

- [ ] **Step 3: Update call sites in `stint-app` and `stint-cli`**

```bash
git grep -n 'SolidtimeClient::new' crates/stint-app crates/stint-cli
```

For each hit: change `SolidtimeClient::new(url, token)` → `SolidtimeClient::with_api_token(url, token)`. Touch nothing else.

- [ ] **Step 4: Update `crates/stint-core/src/sync/push.rs`**

```bash
git grep -n 'SolidtimeClient::new\|self.token' crates/stint-core/src/sync
```

Apply the same `with_api_token` rename to any `SolidtimeClient::new` calls. The internal code in `push.rs` doesn't access `client.token` directly (it uses the public API methods), so no other changes needed.

- [ ] **Step 5: Run the full Rust test suite**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test --workspace -- --test-threads=1
```

Expected: all tests pass. If any failures, fix before committing.

- [ ] **Step 6: Run clippy and fmt**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

- [ ] **Step 7: Commit**

```bash
git add -u
git commit -m "refactor(core): SolidtimeClient holds Arc<dyn TokenProvider>

The token is no longer a static field; every API call resolves a fresh
access-token via the trait. with_api_token() preserves the existing
PAT path with a one-word call-site rename. Behaviour unchanged for
existing callers; positions the client to accept OAuthTokenProvider
(landing in next task)."
```

---

### Task 11: `OAuthTokenProvider` — refresh-on-expiry caching

**Files:**
- Modify: `crates/stint-core/src/solidtime/auth.rs`
- Create: `crates/stint-core/tests/solidtime_oauth_provider.rs`

`OAuthTokenProvider` wraps an `OAuthClient` + an `Arc<Mutex<TokenSet>>`. On every `access_token()` call:
1. Lock the mutex.
2. If `is_expired_with_skew(Utc::now())`, refresh via `OAuthClient::refresh_tokens(&current)`. On success: update the stored set, persist back to Keychain (callback supplied at construction). On failure (Error::OAuthRefreshFailed): propagate so the caller surfaces "please re-authenticate".
3. Return `current.access_token.clone()`.

We pass the Keychain-write callback as a generic `Fn(&TokenSet) -> Result<()>` so the trait stays Keychain-agnostic for testing.

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/solidtime_oauth_provider.rs`:

```rust
use std::sync::{Arc, Mutex};
use std::time::Duration;
use chrono::{Duration as ChronoDuration, Utc};
use stint_core::oauth::client::{OAuthClient, OAuthConfig};
use stint_core::oauth::tokens::TokenSet;
use stint_core::solidtime::auth::{OAuthTokenProvider, TokenProvider};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn oauth_client_for(server: &MockServer) -> OAuthClient {
    OAuthClient::new(OAuthConfig {
        authorize_url: format!("{}/oauth/authorize", server.uri()),
        token_url: format!("{}/oauth/token", server.uri()),
        client_id: "stint-desktop".into(),
        redirect_uri: "http://127.0.0.1:0/callback".into(),
        scopes: vec!["read".into()],
    })
}

#[tokio::test]
async fn returns_cached_access_token_when_not_expired() {
    let server = MockServer::start().await;
    // No mock for /oauth/token — if it gets hit, the test fails with 404.
    let client = oauth_client_for(&server);
    let saved = Arc::new(Mutex::new(None));
    let saved_clone = saved.clone();
    let persist = move |t: &TokenSet| {
        *saved_clone.lock().unwrap() = Some(t.clone());
        Ok(())
    };

    let token_set = TokenSet::from_response(
        "fresh-access".into(),
        Some("r".into()),
        3600,
        None,
        Utc::now(),
    );
    let provider = OAuthTokenProvider::new(client, token_set, Box::new(persist));
    let got = provider.access_token().await.unwrap();
    assert_eq!(got, "fresh-access");
    assert!(saved.lock().unwrap().is_none(), "no persist should have happened — token was fresh");
}

#[tokio::test]
async fn refreshes_and_persists_when_expired() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "token_type": "Bearer",
            "expires_in": 3600,
            "access_token": "refreshed-access",
            "refresh_token": "refreshed-refresh",
            "scope": "read"
        })))
        .mount(&server)
        .await;
    let client = oauth_client_for(&server);
    let saved = Arc::new(Mutex::new(None));
    let saved_clone = saved.clone();
    let persist = move |t: &TokenSet| {
        *saved_clone.lock().unwrap() = Some(t.clone());
        Ok(())
    };

    // Expired 5 min ago.
    let token_set = TokenSet {
        access_token: "stale".into(),
        refresh_token: Some("old-refresh".into()),
        expires_at: Utc::now() - ChronoDuration::minutes(5),
        scope: None,
    };
    let provider = OAuthTokenProvider::new(client, token_set, Box::new(persist));
    let got = provider.access_token().await.unwrap();
    assert_eq!(got, "refreshed-access");

    let saved = saved.lock().unwrap();
    let saved = saved.as_ref().expect("should have persisted");
    assert_eq!(saved.access_token, "refreshed-access");
    assert_eq!(saved.refresh_token.as_deref(), Some("refreshed-refresh"));
}

#[tokio::test]
async fn surfaces_oauth_refresh_failed_when_server_rejects_refresh() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": "invalid_grant"
        })))
        .mount(&server)
        .await;
    let client = oauth_client_for(&server);
    let persist = |_: &TokenSet| Ok(());
    let token_set = TokenSet {
        access_token: "stale".into(),
        refresh_token: Some("expired-refresh".into()),
        expires_at: Utc::now() - ChronoDuration::minutes(5),
        scope: None,
    };
    let provider = OAuthTokenProvider::new(client, token_set, Box::new(persist));
    let err = provider.access_token().await.unwrap_err();
    assert!(matches!(err, stint_core::Error::OAuthRefreshFailed), "got {err:?}");
    // Sleep briefly to allow any deferred wiremock asserts to flush — best practice in case server has not received the request yet.
    tokio::time::sleep(Duration::from_millis(10)).await;
}
```

- [ ] **Step 2: Implement `OAuthTokenProvider`**

Append to `crates/stint-core/src/solidtime/auth.rs`:

```rust
use crate::oauth::client::OAuthClient;
use crate::oauth::tokens::TokenSet;
use chrono::Utc;
use std::sync::Mutex;

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
```

(Adjust the imports at the top of `auth.rs` to include `async_trait` if not already there.)

- [ ] **Step 3: Run test — expect PASS**

```bash
cargo test -p stint-core --test solidtime_oauth_provider
```

Expected: 3 tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/stint-core/src/solidtime/auth.rs \
        crates/stint-core/tests/solidtime_oauth_provider.rs
git commit -m "feat(core): OAuthTokenProvider with refresh-on-expiry caching

Implements TokenProvider for OAuth-backed Solidtime auth. On each
access_token() call, refreshes via OAuthClient if the cached token
is within EXPIRY_SKEW of expiry, then persists the new TokenSet via
the caller-supplied callback (later: Keychain write). Surfaces
OAuthRefreshFailed when the server rejects the refresh-token so
upper layers can prompt re-authentication."
```

---

### Task 12: Keychain helpers — store/load/delete OAuth blob

**Files:**
- Modify: `crates/stint-core/src/solidtime/auth.rs`
- Create: `crates/stint-core/tests/solidtime_oauth_keychain.rs`

Helpers that read/write the OAuth blob as a single JSON entry under `tech.reyem.stint.solidtime.oauth`. We use the existing `Secrets` wrapper from `stint-core::config::secrets`. The blob includes the client_id + the TokenSet.

- [ ] **Step 1: Write the failing test (skipped in CI like the other Keychain tests)**

Create `crates/stint-core/tests/solidtime_oauth_keychain.rs`:

```rust
use chrono::Utc;
use stint_core::config::secrets::Secrets;
use stint_core::oauth::tokens::TokenSet;
use stint_core::solidtime::auth::{oauth_blob_load, oauth_blob_save, oauth_blob_delete, OAuthBlob};

fn unique_secrets() -> Secrets {
    Secrets::with_service_prefix(format!("tech.reyem.stint.test-{}", uuid::Uuid::new_v4()))
}

#[test]
fn round_trips_blob_through_keychain() {
    if std::env::var("STINT_SKIP_KEYCHAIN_TESTS").is_ok() {
        eprintln!("skipping: STINT_SKIP_KEYCHAIN_TESTS is set");
        return;
    }
    let secrets = unique_secrets();

    assert!(oauth_blob_load(&secrets).unwrap().is_none());
    let tokens = TokenSet::from_response("a".into(), Some("r".into()), 3600, Some("read".into()), Utc::now());
    let blob = OAuthBlob {
        client_id: "stint-desktop".into(),
        tokens,
    };
    oauth_blob_save(&secrets, &blob).unwrap();

    let loaded = oauth_blob_load(&secrets).unwrap().expect("present");
    assert_eq!(loaded.client_id, "stint-desktop");
    assert_eq!(loaded.tokens.access_token, "a");

    oauth_blob_delete(&secrets).unwrap();
    assert!(oauth_blob_load(&secrets).unwrap().is_none());
}
```

- [ ] **Step 2: Implement helpers**

Append to `crates/stint-core/src/solidtime/auth.rs`:

```rust
use crate::config::secrets::Secrets;
use crate::oauth::tokens::TokenSet;
use serde::{Deserialize, Serialize};

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
```

- [ ] **Step 3: Verify**

```bash
cargo test -p stint-core --test solidtime_oauth_keychain -- --nocapture
```

Expected: PASS locally. The test is env-gated, so CI will skip it.

- [ ] **Step 4: Commit**

```bash
git add crates/stint-core/src/solidtime/auth.rs \
        crates/stint-core/tests/solidtime_oauth_keychain.rs
git commit -m "feat(core): Keychain helpers for OAuth blob

oauth_blob_save / oauth_blob_load / oauth_blob_delete persist the
(client_id, TokenSet) tuple as a single JSON blob under
tech.reyem.stint.solidtime.oauth. Single-blob layout avoids the
window where access/refresh/expiry could be inconsistent across
multiple Keychain entries during a token rotation."
```

---

### Task 13: `solidtime.auth_mode` settings flag + auth resolver

**Files:**
- Modify: `crates/stint-core/src/solidtime/auth.rs`
- Create: `crates/stint-core/tests/solidtime_auth_resolver.rs`

A high-level helper that reads `solidtime.auth_mode` from settings + the appropriate credential from Keychain, and returns a fully-wired `Arc<dyn TokenProvider>` plus an `OAuthClient` (for surfaces that need to trigger a login flow). Called by CLI and Tauri commands at startup-of-API-call time.

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/solidtime_auth_resolver.rs`:

```rust
mod common;

use stint_core::config::secrets::Secrets;
use stint_core::config::Settings;
use stint_core::solidtime::auth::{build_token_provider, AuthMode};

#[tokio::test]
async fn returns_api_token_provider_when_mode_is_api_token() {
    if std::env::var("STINT_SKIP_KEYCHAIN_TESTS").is_ok() {
        eprintln!("skipping: STINT_SKIP_KEYCHAIN_TESTS is set");
        return;
    }
    let env = common::setup().await;
    Settings::new(env.store.clone())
        .set("solidtime.auth_mode", "api_token")
        .await
        .unwrap();
    let secrets = Secrets::with_service_prefix(format!(
        "tech.reyem.stint.test-{}",
        uuid::Uuid::new_v4()
    ));
    secrets.set("solidtime", "the-pat-token").unwrap();

    let (provider, _client) = build_token_provider(
        &Settings::new(env.store.clone()),
        &secrets,
        "https://time.example.com",
    )
    .await
    .unwrap();
    assert_eq!(provider.access_token().await.unwrap(), "the-pat-token");
    secrets.delete("solidtime").unwrap();
}

#[tokio::test]
async fn returns_missing_config_when_oauth_mode_but_no_blob() {
    let env = common::setup().await;
    Settings::new(env.store.clone())
        .set("solidtime.auth_mode", "oauth")
        .await
        .unwrap();
    let secrets = Secrets::with_service_prefix(format!(
        "tech.reyem.stint.test-{}",
        uuid::Uuid::new_v4()
    ));

    let err = build_token_provider(
        &Settings::new(env.store.clone()),
        &secrets,
        "https://time.example.com",
    )
    .await
    .unwrap_err();
    match err {
        stint_core::Error::MissingConfig(k) => {
            assert_eq!(k, "solidtime.oauth");
        }
        e => panic!("expected MissingConfig, got {e:?}"),
    }
}
```

- [ ] **Step 2: Implement the resolver**

Append to `crates/stint-core/src/solidtime/auth.rs`:

```rust
use crate::config::Settings;
use crate::oauth::client::{OAuthClient, OAuthConfig};
use std::sync::Arc;

const AUTH_MODE_KEY: &str = "solidtime.auth_mode";
const API_TOKEN_KEYCHAIN_KEY: &str = "solidtime";

const DEFAULT_SCOPES: &[&str] = &["read", "create", "update", "delete"];
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
    let mode = AuthMode::from_str_or_default(
        settings.get(AUTH_MODE_KEY).await?.as_deref(),
    );

    let blob = oauth_blob_load(secrets)?;
    let client_id = blob
        .as_ref()
        .map(|b| b.client_id.clone())
        .unwrap_or_else(|| "stint-desktop".to_string());

    let oauth_client = OAuthClient::new(OAuthConfig {
        authorize_url: format!("{}/oauth/authorize", solidtime_base_url.trim_end_matches('/')),
        token_url: format!("{}/oauth/token", solidtime_base_url.trim_end_matches('/')),
        client_id,
        redirect_uri: DEFAULT_REDIRECT_URI_HOST.into(), // placeholder; real port assigned at login time
        scopes: DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect(),
    });

    match mode {
        AuthMode::ApiToken => {
            let token = secrets
                .get(API_TOKEN_KEYCHAIN_KEY)?
                .ok_or(crate::Error::MissingConfig("solidtime"))?;
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
```

NOTE: `Secrets` needs a `Clone` impl for the `persist` closure capture. Check `crates/stint-core/src/config/secrets.rs` — if `Clone` isn't derived, add `#[derive(Clone)]` on the `Secrets` struct in a single-line edit.

- [ ] **Step 3: Run test**

```bash
cargo test -p stint-core --test solidtime_auth_resolver
```

Expected: PASS (with STINT_SKIP_KEYCHAIN_TESTS unset for the api_token test, set for skip-mode).

- [ ] **Step 4: Commit**

```bash
git add crates/stint-core/src/solidtime/auth.rs \
        crates/stint-core/src/config/secrets.rs \
        crates/stint-core/tests/solidtime_auth_resolver.rs
git commit -m "feat(core): build_token_provider resolves auth_mode → TokenProvider

Reads solidtime.auth_mode and the matching credential, wires up either
ApiTokenProvider or OAuthTokenProvider, and returns it alongside an
OAuthClient configured for the user's Solidtime instance. The persist
callback for OAuthTokenProvider points back at oauth_blob_save so
refreshes are written atomically to Keychain."
```

---

### Task 14: End-to-end OAuth login helper

**Files:**
- Modify: `crates/stint-core/src/solidtime/auth.rs`
- Create: `crates/stint-core/tests/solidtime_login_e2e.rs`

A high-level "run the whole interactive flow" function that callers (CLI + Tauri) use. Returns the captured `TokenSet`, lets the caller persist it. The test drives the whole flow against wiremock + a simulated browser hit.

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/solidtime_login_e2e.rs`:

```rust
use std::time::Duration;
use stint_core::oauth::client::{OAuthClient, OAuthConfig};
use stint_core::solidtime::auth::login_interactive;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn interactive_login_completes_against_mock_authz_server() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "token_type": "Bearer",
            "expires_in": 3600,
            "access_token": "first-access",
            "refresh_token": "first-refresh",
            "scope": "read create update delete"
        })))
        .mount(&server)
        .await;

    let client = OAuthClient::new(OAuthConfig {
        authorize_url: format!("{}/oauth/authorize", server.uri()),
        token_url: format!("{}/oauth/token", server.uri()),
        client_id: "stint-desktop".into(),
        redirect_uri: "http://127.0.0.1:0/callback".into(),
        scopes: vec!["read".into(), "create".into(), "update".into(), "delete".into()],
    });

    // Simulate the browser hitting the callback in the background.
    let browser_simulator = |authorize_url: String| {
        // The callback URL is encoded as `redirect_uri` in the authorize URL's query.
        // The CSRF state is also in the query — we need to round-trip it back unchanged.
        tokio::spawn(async move {
            // Parse the redirect_uri + state from the authorize URL.
            let parsed = url::Url::parse(&authorize_url).unwrap();
            let mut state = None;
            let mut redirect = None;
            for (k, v) in parsed.query_pairs() {
                match k.as_ref() {
                    "state" => state = Some(v.into_owned()),
                    "redirect_uri" => redirect = Some(v.into_owned()),
                    _ => {}
                }
            }
            let redirect = redirect.expect("authorize URL has redirect_uri");
            let state = state.expect("authorize URL has state");
            // Wait briefly so the loopback server has a chance to start accepting.
            tokio::time::sleep(Duration::from_millis(50)).await;
            let callback = format!("{redirect}?code=ok-code&state={state}");
            let _ = reqwest::get(&callback).await;
        });
    };

    let tokens = login_interactive(&client, Duration::from_secs(10), browser_simulator)
        .await
        .unwrap();
    assert_eq!(tokens.access_token, "first-access");
    assert_eq!(tokens.refresh_token.as_deref(), Some("first-refresh"));
}
```

- [ ] **Step 2: Implement `login_interactive`**

Append to `crates/stint-core/src/solidtime/auth.rs`:

```rust
use crate::oauth::loopback::listen_for_callback;
use std::time::Duration;

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
```

- [ ] **Step 3: Run test**

```bash
cargo test -p stint-core --test solidtime_login_e2e
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/stint-core/src/solidtime/auth.rs \
        crates/stint-core/tests/solidtime_login_e2e.rs
git commit -m "feat(core): login_interactive — full PKCE flow helper

Spins up loopback server, mints authorize URL with the bound port,
invokes the caller-supplied open_browser closure, awaits the redirect,
verifies CSRF state, exchanges code → TokenSet. Caller persists.
The test simulates the browser via a tokio task that hits the
loopback callback URL with code+state."
```

---

### Task 15: CLI `stint config login` and `stint config logout`

**Files:**
- Modify: `crates/stint-cli/src/main.rs`
- Create: `crates/stint-cli/src/config_login.rs`
- Create: `crates/stint-cli/tests/cli_login.rs`

`stint config login` runs the OAuth flow, persists the TokenSet to Keychain, and sets `solidtime.auth_mode=oauth`. `stint config logout` deletes the OAuth blob and sets `solidtime.auth_mode=api_token` if a PAT exists in Keychain (otherwise leaves it as `oauth` so the user can re-login).

- [ ] **Step 1: Read the current CLI command structure**

```bash
grep -n "ConfigCmd\|enum Config\|fn config" crates/stint-cli/src/main.rs | head -10
```

Find where `Config` subcommands are defined. The new `Login` and `Logout` variants go alongside `Set`, `Show`, `Test`.

- [ ] **Step 2: Add the `Login` and `Logout` clap subcommands**

Locate the `enum ConfigCmd { ... }` (name may vary; use what's there). Add two variants:

```rust
    /// Run OAuth 2.0 PKCE login against the configured Solidtime instance.
    Login,
    /// Remove the OAuth token blob from Keychain.
    Logout,
```

In the matcher for those variants, dispatch to `config_login::run_login(...)` and `config_login::run_logout(...)`. Add `mod config_login;` at the top of `main.rs`.

- [ ] **Step 3: Implement the CLI handlers**

Create `crates/stint-cli/src/config_login.rs`:

```rust
use anyhow::{anyhow, Context, Result};
use std::time::Duration;
use stint_core::config::secrets::Secrets;
use stint_core::config::Settings;
use stint_core::oauth::client::{OAuthClient, OAuthConfig};
use stint_core::solidtime::auth::{
    login_interactive, oauth_blob_delete, oauth_blob_save, OAuthBlob,
};
use stint_core::store::Store;

const FLOW_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes

pub async fn run_login(store: Store) -> Result<()> {
    let settings = Settings::new(store.clone());
    let base_url = settings
        .get("solidtime.url")
        .await?
        .ok_or_else(|| anyhow!("solidtime.url is not set; run `stint config set solidtime.url <URL>` first"))?;

    let client_id = match settings.get("solidtime.oauth.client_id").await? {
        Some(id) => id,
        None => {
            eprintln!("solidtime.oauth.client_id is not set.");
            eprintln!("Register an OAuth client on your Solidtime instance (see README), then run:");
            eprintln!("  stint config set solidtime.oauth.client_id <CLIENT-ID>");
            return Err(anyhow!("missing OAuth client ID"));
        }
    };

    let secrets = Secrets::default();
    let client = OAuthClient::new(OAuthConfig {
        authorize_url: format!("{}/oauth/authorize", base_url.trim_end_matches('/')),
        token_url: format!("{}/oauth/token", base_url.trim_end_matches('/')),
        client_id: client_id.clone(),
        redirect_uri: "http://127.0.0.1:0/callback".into(),
        scopes: vec!["read".into(), "create".into(), "update".into(), "delete".into()],
    });

    println!("Opening browser to sign in to {base_url}...");
    let tokens = login_interactive(&client, FLOW_TIMEOUT, |url| {
        if let Err(e) = webbrowser::open(&url) {
            eprintln!("Could not open browser ({e}). Please visit:\n  {url}");
        }
    })
    .await
    .context("OAuth flow failed")?;

    let blob = OAuthBlob {
        client_id,
        tokens,
    };
    oauth_blob_save(&secrets, &blob).context("persist OAuth blob")?;
    settings.set("solidtime.auth_mode", "oauth").await?;
    println!("✓ Signed in. solidtime.auth_mode is now 'oauth'.");
    Ok(())
}

pub async fn run_logout(store: Store) -> Result<()> {
    let settings = Settings::new(store);
    let secrets = Secrets::default();
    oauth_blob_delete(&secrets).context("delete OAuth blob")?;

    // If a PAT exists, fall back to it. Otherwise leave auth_mode=oauth so the
    // next `stint config login` doesn't have to also set the mode.
    if secrets.get("solidtime")?.is_some() {
        settings.set("solidtime.auth_mode", "api_token").await?;
        println!("✓ OAuth tokens cleared. Falling back to the stored API token.");
    } else {
        println!("✓ OAuth tokens cleared. Run `stint config set solidtime.token` or `stint config login` to re-authenticate.");
    }
    Ok(())
}
```

- [ ] **Step 4: Smoke test from the terminal (manual)**

The full CLI dance requires a real Solidtime instance and registered client. For now run the help to confirm the subcommands wired up:

```bash
cargo run -p stint-cli -- config login --help
cargo run -p stint-cli -- config logout --help
```

Expected: both print clap-generated help, no errors.

- [ ] **Step 5: Write an integration test that exercises `logout` without a network**

Create `crates/stint-cli/tests/cli_login.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn logout_with_no_oauth_blob_completes_cleanly() {
    let tempdir = tempfile::tempdir().unwrap();
    Command::cargo_bin("stint")
        .unwrap()
        .env("STINT_DB_DIR", tempdir.path())
        .env("STINT_SKIP_KEYCHAIN_TESTS", "1")
        .args(["config", "logout"])
        .assert()
        .success()
        .stdout(contains("OAuth tokens cleared"));
}
```

NOTE: `STINT_DB_DIR` is the env var stint uses to override the default DB path; verify the name in `crates/stint-core/src/paths.rs`. If the var name differs, use the actual one.

- [ ] **Step 6: Run the test**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-cli --test cli_login
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/stint-cli/src crates/stint-cli/tests/cli_login.rs
git commit -m "feat(cli): stint config login + stint config logout

login: interactive OAuth PKCE flow via system browser, persists the
TokenSet to Keychain, flips solidtime.auth_mode to 'oauth'.
logout: deletes the OAuth blob; if a PAT is still in Keychain,
flips auth_mode back to 'api_token'.
Settings.solidtime.oauth.client_id must be set first; the command
prints actionable guidance otherwise."
```

---

### Task 16: Tauri commands for the GUI flow

**Files:**
- Modify: `crates/stint-app/src/commands/config.rs`
- Modify: `crates/stint-app/src/main.rs` (register new commands in `invoke_handler!`)

Three Tauri commands the UI calls:

- `oauth_solidtime_status() -> { mode: "api_token" | "oauth", signed_in: bool, scope: Option<String> }`
- `oauth_solidtime_start() -> Result<()>` — runs `login_interactive`, persists, sets `auth_mode=oauth`.
- `oauth_solidtime_logout() -> Result<()>` — wraps `run_logout`.

- [ ] **Step 1: Add the commands**

In `crates/stint-app/src/commands/config.rs`, after the existing commands, add:

```rust
use serde::Serialize;
use std::time::Duration;
use stint_core::config::secrets::Secrets;
use stint_core::oauth::client::{OAuthClient, OAuthConfig};
use stint_core::solidtime::auth::{
    login_interactive, oauth_blob_delete, oauth_blob_load, oauth_blob_save, AuthMode, OAuthBlob,
};

#[derive(Serialize)]
pub struct SolidtimeAuthStatus {
    mode: &'static str,
    signed_in: bool,
    scope: Option<String>,
}

#[tauri::command]
pub async fn oauth_solidtime_status(
    state: State<'_, RwLock<AppState>>,
) -> Result<SolidtimeAuthStatus, AppError> {
    let store = store(&state).await;
    let settings = Settings::new((*store).clone());
    let mode = AuthMode::from_str_or_default(settings.get("solidtime.auth_mode").await?.as_deref());
    let secrets = Secrets::default();
    let (signed_in, scope) = match mode {
        AuthMode::ApiToken => (secrets.get("solidtime")?.is_some(), None),
        AuthMode::OAuth => {
            let blob = oauth_blob_load(&secrets)?;
            (
                blob.is_some(),
                blob.and_then(|b| b.tokens.scope),
            )
        }
    };
    Ok(SolidtimeAuthStatus {
        mode: match mode {
            AuthMode::ApiToken => "api_token",
            AuthMode::OAuth => "oauth",
        },
        signed_in,
        scope,
    })
}

#[tauri::command]
pub async fn oauth_solidtime_start(
    state: State<'_, RwLock<AppState>>,
) -> Result<(), AppError> {
    let store = store(&state).await;
    let settings = Settings::new((*store).clone());
    let base_url = settings
        .get("solidtime.url")
        .await?
        .ok_or(AppError::msg("solidtime.url is not set"))?;
    let client_id = settings
        .get("solidtime.oauth.client_id")
        .await?
        .ok_or(AppError::msg("solidtime.oauth.client_id is not set"))?;

    let client = OAuthClient::new(OAuthConfig {
        authorize_url: format!("{}/oauth/authorize", base_url.trim_end_matches('/')),
        token_url: format!("{}/oauth/token", base_url.trim_end_matches('/')),
        client_id: client_id.clone(),
        redirect_uri: "http://127.0.0.1:0/callback".into(),
        scopes: vec!["read".into(), "create".into(), "update".into(), "delete".into()],
    });

    let tokens = login_interactive(&client, Duration::from_secs(300), |url| {
        let _ = tauri_plugin_opener::open_url(&url, None::<&str>);
    })
    .await
    .map_err(AppError::from)?;

    oauth_blob_save(&Secrets::default(), &OAuthBlob { client_id, tokens })?;
    settings.set("solidtime.auth_mode", "oauth").await?;
    Ok(())
}

#[tauri::command]
pub async fn oauth_solidtime_logout(
    state: State<'_, RwLock<AppState>>,
) -> Result<(), AppError> {
    let store = store(&state).await;
    let settings = Settings::new((*store).clone());
    let secrets = Secrets::default();
    oauth_blob_delete(&secrets)?;
    if secrets.get("solidtime")?.is_some() {
        settings.set("solidtime.auth_mode", "api_token").await?;
    }
    Ok(())
}
```

NOTE: `AppError::msg` may need to be defined if it doesn't already exist — check `crates/stint-app/src/error.rs` (or wherever `AppError` lives) and add a small `pub fn msg(s: &str) -> Self` constructor if needed. Use Edit to add it.

The `tauri_plugin_opener::open_url` signature may differ; verify against the installed version. Worst case fall back to spawning `open ${url}` via `std::process::Command` on macOS.

- [ ] **Step 2: Register the new commands in `main.rs`**

In `crates/stint-app/src/main.rs`, add to the `invoke_handler![...]` list:

```rust
            commands::config::oauth_solidtime_status,
            commands::config::oauth_solidtime_start,
            commands::config::oauth_solidtime_logout,
```

- [ ] **Step 3: Verify compile**

```bash
cargo check -p stint-app
```

Expected: clean. Fix any signature mismatches that surface.

- [ ] **Step 4: Commit**

```bash
git add crates/stint-app/src/commands/config.rs crates/stint-app/src/main.rs \
        crates/stint-app/src/error.rs
git commit -m "feat(app): tauri commands for Solidtime OAuth status/start/logout

oauth_solidtime_status returns the current auth mode + whether the user
is signed in. oauth_solidtime_start runs the interactive PKCE flow,
opens the browser via tauri-plugin-opener, persists the TokenSet.
oauth_solidtime_logout clears the OAuth blob; if a PAT exists,
falls back to api_token mode."
```

---

### Task 17: GUI Settings UI — auth method radio + Sign in button

**Files:**
- Modify: `ui/src/routes/Settings.tsx`
- Modify: `ui/src/api.ts` (typed wrappers around the new Tauri commands)
- Modify: `ui/src/types.ts` (if separate types file exists; otherwise inline)

UI additions:
- A radio group: **API token** vs **OAuth (Sign in with Solidtime)**.
- When **API token** is selected: existing token input field stays as-is.
- When **OAuth** is selected: show a "Sign in with Solidtime" button + a status pill ("Not signed in" / "Signed in (scope: read create update delete)").
- A `client_id` text input (required before Sign in).
- After sign-in, status pill flips; user can click "Sign out" to revoke.

- [ ] **Step 1: Add typed wrappers in `ui/src/api.ts`**

Find the existing wrappers (probably `invoke('solidtime_url', ...)` etc.) and add:

```ts
import { invoke } from '@tauri-apps/api/core';

export type SolidtimeAuthStatus = {
  mode: 'api_token' | 'oauth';
  signed_in: boolean;
  scope: string | null;
};

export const oauthSolidtimeStatus = () =>
  invoke<SolidtimeAuthStatus>('oauth_solidtime_status');

export const oauthSolidtimeStart = () =>
  invoke<void>('oauth_solidtime_start');

export const oauthSolidtimeLogout = () =>
  invoke<void>('oauth_solidtime_logout');
```

- [ ] **Step 2: Add the radio + button to `Settings.tsx`**

Read the existing Settings route to find the Solidtime configuration section. Insert a new sub-section above the API-token field, e.g.:

```tsx
import { createSignal, createResource, Show } from 'solid-js';
import { oauthSolidtimeStatus, oauthSolidtimeStart, oauthSolidtimeLogout } from '~/api';

// ... inside the component
const [authStatus, { refetch }] = createResource(() => oauthSolidtimeStatus());
const [authMode, setAuthMode] = createSignal<'api_token' | 'oauth'>('api_token');

// Sync local radio with backend state on first load
createEffect(() => {
  const s = authStatus();
  if (s) setAuthMode(s.mode);
});

const handleSignIn = async () => {
  await oauthSolidtimeStart();
  await refetch();
};
const handleSignOut = async () => {
  await oauthSolidtimeLogout();
  await refetch();
};
```

And in the JSX, before the existing API-token block:

```tsx
<section class="space-y-2">
  <label class="text-sm font-semibold">Authentication method</label>
  <div class="flex gap-4">
    <label class="flex items-center gap-2">
      <input
        type="radio"
        name="auth_mode"
        value="api_token"
        checked={authMode() === 'api_token'}
        onChange={() => setAuthMode('api_token')}
      />
      <span>API token</span>
    </label>
    <label class="flex items-center gap-2">
      <input
        type="radio"
        name="auth_mode"
        value="oauth"
        checked={authMode() === 'oauth'}
        onChange={() => setAuthMode('oauth')}
      />
      <span>Sign in with Solidtime (OAuth)</span>
    </label>
  </div>

  <Show when={authMode() === 'oauth'}>
    <div class="space-y-2 rounded-md border border-neutral-200 p-3">
      <Show when={authStatus()?.signed_in} fallback={
        <button class="rounded bg-blue-600 px-3 py-1.5 text-white" onClick={handleSignIn}>
          Sign in with Solidtime
        </button>
      }>
        <div class="flex items-center gap-2 text-sm">
          <span class="rounded-full bg-green-100 px-2 py-0.5 text-green-800">Signed in</span>
          <Show when={authStatus()?.scope}>
            <span class="text-neutral-600">scope: {authStatus()?.scope}</span>
          </Show>
          <button class="ml-auto text-blue-600 hover:underline" onClick={handleSignOut}>
            Sign out
          </button>
        </div>
      </Show>
    </div>
  </Show>
</section>
```

- [ ] **Step 3: Verify the UI typechecks and builds**

```bash
pnpm -C ui typecheck
pnpm -C ui build
```

Expected: clean. If the existing Settings.tsx has its own form-state hooks, integrate rather than introduce parallel state.

- [ ] **Step 4: Manual UI smoke test (visual, in dev mode)**

```bash
cd crates/stint-app && cargo tauri dev
```

Then in the running app: open Settings, confirm the radio renders, switch between modes, confirm the Sign in button shows under OAuth. (You won't actually be able to complete the OAuth flow until a `client_id` is set; that's expected.)

- [ ] **Step 5: Commit**

```bash
git add ui/src/api.ts ui/src/routes/Settings.tsx ui/src/types.ts
git commit -m "feat(ui): Settings auth-method radio + Sign in with Solidtime

Adds an authentication-method radio group to the Settings panel.
When OAuth is selected: shows a 'Signed in' status pill (scope echo)
with a Sign out action, or a 'Sign in with Solidtime' button that
triggers oauth_solidtime_start (opens the system browser via
tauri-plugin-opener). State refetched after sign-in / sign-out."
```

---

### Task 18: Documentation — README + CLAUDE.md + AGENTS.md

**Files:**
- Modify: `README.md`
- Modify: `CLAUDE.md`
- Modify: `AGENTS.md`

Two doc additions:
1. A "Sign in with Solidtime" section in README under setup, including the `php artisan` snippet to register a client.
2. A gotcha in CLAUDE.md/AGENTS.md about the OAuth blob location + the `solidtime.oauth.client_id` settings key.

The README section should be self-contained (a user who has never seen OAuth can follow it).

- [ ] **Step 1: Add to `README.md`**

In the "Setup" or equivalent section (read the file to find the right place), insert a new subsection:

````markdown
### Signing in with Solidtime OAuth (optional, alternative to API token)

stint supports OAuth 2.0 PKCE against your self-hosted Solidtime instance, in addition to the existing personal-access-token flow. The OAuth path lets the access-token rotate automatically (refresh-tokens stored in Keychain), but requires a one-time OAuth client registration on your Solidtime server.

**1. Register an OAuth client on your Solidtime instance.** SSH into the host running Solidtime and run:

```bash
php artisan passport:client \
    --public \
    --name="stint" \
    --redirect_uri="http://127.0.0.1/callback"
```

Note the **Client ID** that's printed. (The wildcard port in the redirect URI is fine — Passport allows loopback redirect URIs to vary by port at runtime.)

**2. Tell stint about the client ID.**

```bash
stint config set solidtime.oauth.client_id <THE-CLIENT-ID>
```

Or in the GUI: Settings → Authentication method → OAuth → fill in **Client ID**.

**3. Sign in.**

CLI: `stint config login`. GUI: Settings → click **Sign in with Solidtime**.

A browser opens, you authenticate against Solidtime, and stint captures the redirect on a random loopback port. After this point, `solidtime.auth_mode` is `oauth`, and refresh-tokens rotate transparently.

To switch back to API token: `stint config logout` (if you still have a PAT in Keychain it becomes active again), or pick **API token** in Settings.
````

- [ ] **Step 2: Update the phase table in README**

Flip the Phase 3a row to shipped:

```markdown
| 3a | OAuth 2.0 foundation + Solidtime OAuth | ✅ shipped (`phase-3a-complete`) |
```

(Match the existing row format.)

- [ ] **Step 3: Add two gotchas to `CLAUDE.md`**

In the "Gotchas / dev-environment notes" section, append:

```markdown
- **OAuth tokens are one Keychain entry, not three.** Solidtime OAuth
  refresh/access/expiry are persisted as a single JSON blob under
  `tech.reyem.stint.solidtime.oauth`. The blob is rewritten atomically
  on every refresh. The legacy PAT entry at `tech.reyem.stint.solidtime`
  is independent — both can coexist; `solidtime.auth_mode` settings key
  picks which is active. The OAuth `client_id` is non-secret and lives
  in the same blob (and is mirrored to the `solidtime.oauth.client_id`
  settings key for first-time setup).
- **OAuth flow needs a registered client on Solidtime.** There's no
  public client-registration UI; users must run `php artisan
  passport:client --public --name=stint --redirect_uri=http://127.0.0.1/callback`
  on their Solidtime host. See the README "Signing in with Solidtime OAuth"
  section for the full setup.
```

Update the "When you start work on a phase" section's example branch name from `phase-N` to `phase-N` (no change), but add Phase 3a to the roadmap table:

```markdown
| 3a | OAuth 2.0 foundation + Solidtime OAuth | ✅ shipped (`phase-3a-complete`) |
```

- [ ] **Step 4: Mirror the CLAUDE.md gotchas into AGENTS.md**

`AGENTS.md` is a pointer file per the Phase 2.5 lessons-learned. If it stays a pointer, no content changes are needed. If you want AGENTS.md to mirror CLAUDE.md for non-Claude consumers, that's a larger refactor — out of scope here. Just verify AGENTS.md still redirects to CLAUDE.md correctly.

```bash
cat AGENTS.md
```

If it's a pointer: leave it. If it's been turned into a duplicate since Phase 2.5: apply the same gotcha additions.

- [ ] **Step 5: Verify all three files**

```bash
grep -n "phase-3a\|3a |\|Phase 3a" README.md CLAUDE.md AGENTS.md
```

Confirm the new section appears in README, the new gotchas appear in CLAUDE.md, and the phase row is updated.

- [ ] **Step 6: Commit**

```bash
git add README.md CLAUDE.md AGENTS.md
git commit -m "docs: Phase 3a OAuth setup + gotchas + roadmap update

README gets a 'Signing in with Solidtime OAuth' section with the
php artisan passport:client snippet for one-time client registration.
CLAUDE.md gets two new gotchas: the single-Keychain-blob layout and
the manual client-registration requirement. Phase 3a row flipped
to shipped in both files."
```

---

### Task 19: Final verification + PR open + merge + tag

- [ ] **Step 1: Run the full local check**

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test --workspace -- --test-threads=1
pnpm install --frozen-lockfile
pnpm -C ui install --frozen-lockfile
pnpm -C ui typecheck
pnpm -C ui build
```

Expected: every command exits zero. Fix anything that doesn't before opening the PR.

- [ ] **Step 2: Push the branch and open the PR (mark ready, not draft)**

```bash
git push -u origin phase-3a
gh pr create --base main --head phase-3a \
  --title "Phase 3a: OAuth 2.0 foundation + Solidtime OAuth" \
  --body "$(cat <<'EOF'
## Summary
- Shared OAuth 2.0 PKCE machinery in `stint-core::oauth` (PKCE, loopback redirect server, token exchange + refresh)
- `TokenProvider` trait + `ApiTokenProvider` + `OAuthTokenProvider` (refresh-on-expiry caching)
- `SolidtimeClient` now holds `Arc<dyn TokenProvider>`; `with_api_token` preserves the existing call shape
- `solidtime.auth_mode` settings key (api_token | oauth)
- CLI: `stint config login` / `stint config logout`
- GUI: Settings auth-method radio + Sign in with Solidtime
- README: documented `php artisan passport:client` registration step
- CLAUDE.md / AGENTS.md: new gotchas, Phase 3a row flipped to shipped

## Test plan
- [ ] CI green
- [ ] `cargo test --workspace -- --test-threads=1` passes locally with and without `STINT_SKIP_KEYCHAIN_TESTS=1`
- [ ] Manual: `stint config login` against a real Solidtime instance opens browser, captures redirect, persists tokens
- [ ] Manual: GUI Settings → OAuth → Sign in completes end-to-end
- [ ] After sign-in, `stint today` continues to work (proves `OAuthTokenProvider` is correctly wired into `SolidtimeClient`)
- [ ] `stint config logout` clears the blob and falls back to PAT if present
EOF
)"
```

- [ ] **Step 3: Watch CI to green**

```bash
gh run watch $(gh run list --branch phase-3a --limit 1 --json databaseId --jq '.[0].databaseId') --exit-status
```

Iterate with `fix(ci):` / `fix(core):` / `fix(*):` commits as needed. NO amend cascades — one new commit per iteration.

- [ ] **Step 4: Manual end-to-end verification**

Against a real Solidtime instance with a registered OAuth client:

```bash
# Pre-flight
stint config show
stint config set solidtime.oauth.client_id <THE-CLIENT-ID>
stint config login
# Browser opens; complete Solidtime sign-in
# Terminal prints "✓ Signed in. solidtime.auth_mode is now 'oauth'."
stint today
# Should print today's entries — proves the OAuth token flows through to SolidtimeClient.
stint config logout
# Falls back to PAT if present, or prompts re-login
```

Record observed behaviour in the PR conversation as test-plan evidence.

- [ ] **Step 5: Rebase-merge the PR**

```bash
gh pr merge --rebase --delete-branch
```

- [ ] **Step 6: Tag `phase-3a-complete`**

```bash
git checkout main
git pull
git tag -a phase-3a-complete -m "Phase 3a: OAuth 2.0 foundation + Solidtime OAuth"
git push origin phase-3a-complete
```

---

## Self-Review

**1. Spec coverage (§5 of the design):**

| Spec requirement | Plan task |
|---|---|
| `OAuthClient` wrapper around the `oauth2` crate | Task 5 + 7 + 8 |
| Common redirect-capture HTTP server bound to `127.0.0.1:<random>` | Task 6 |
| Common refresh loop that writes refreshed tokens back to Keychain | Task 11 + 12 |
| API token continues to work; OAuth added alongside | Task 9 (ApiTokenProvider) + Task 10 (refactor) + Task 13 (mode flag) |
| `SolidtimeAuth` enum so the rest of `stint-core` doesn't care | Materialised as `TokenProvider` trait (Task 9) — see "Why a `TokenProvider` trait" sidebar |
| OAuth tokens land in Keychain under `tech.reyem.stint.solidtime.oauth.*` | Task 12 (one blob, atomic rotation — divergence from spec noted) |
| CLI surface for sign-in | Task 15 |
| GUI surface for sign-in | Task 16 + 17 |
| Documentation | Task 18 |

**Spec divergences (intentional, documented in sidebars):**
- `SolidtimeAuth` enum → `TokenProvider` trait (Task 9 sidebar)
- Multiple `tech.reyem.stint.solidtime.oauth.*` entries → single `tech.reyem.stint.solidtime.oauth` JSON blob (sidebar at top + Task 12)
- Auto-update of `oauth2` crate version: spec doesn't pin; plan uses v5.

**2. Placeholder scan:**

Search this plan for `TODO`, `TBD`, `fill in`, `appropriate error handling`, `similar to Task`. Self-check after writing — see the corresponding section of the writing-plans skill.

**3. Type / name consistency:**

- `TokenSet` defined Task 3, used Task 7/8/11/12/14.
- `OAuthClient`/`OAuthConfig` defined Task 5, used Task 7/8/11/13/14.
- `TokenProvider` trait + `ApiTokenProvider`/`OAuthTokenProvider` defined Tasks 9/11, used Task 10/13/15/16.
- `OAuthBlob` defined Task 12, used Task 13/15/16.
- `oauth_blob_load` / `oauth_blob_save` / `oauth_blob_delete` defined Task 12, used Tasks 13/15/16.
- `build_token_provider` / `login_interactive` / `AuthMode` defined Tasks 13/14, used Task 15/16.
- Settings key `solidtime.auth_mode` defined Task 13, used Task 15/16.
- Keychain key `solidtime.oauth` (via `OAUTH_KEYCHAIN_KEY`) defined Task 12.
- Settings key `solidtime.oauth.client_id` introduced Task 15, used Task 16, documented Task 18.

**4. Lessons-from-previous-phases applied:**

- Pre-flight: Task 19 step 1 runs all CI commands locally before opening the PR — catches drift Phase 2.5-style.
- No amend cascades on CI failures — Task 19 step 3 explicitly forbids.
- Smoke-from-fresh-branch is unnecessary now (CI is already live on main from Phase 2.5).
- Plan-doc lives at `docs/superpowers/plans/`; same convention as prior phases.

**5. Risks worth flagging during execution:**

- **`oauth2` v5 API may have changed** since this plan was written. If `BasicClient::new` signature or feature flags differ, expect 1–2 `fix(deps):` commits to align.
- **`tauri-plugin-opener` API** for `open_url` is referenced in Task 16. Confirm against the installed version; fall back to spawning `/usr/bin/open <url>` via `std::process::Command` if needed.
- **Solidtime's `php artisan passport:client`** may require additional flags depending on Passport version. If `--public` isn't recognised, Passport's docs are the source of truth.
- **Refresh-token rotation behaviour on Solidtime** — Passport may not actually rotate refresh-tokens on every refresh (depends on Passport config). `TokenSet::merge_refresh_response` is designed to handle both cases, but verify against the real instance during Task 19 step 4.

---

## Execution Handoff

Plan covers ~19 tasks across `stint-core` (12), `stint-cli` (1), `stint-app` (1), `ui/` (1), docs (1), and wrap-up (3). The OAuth machinery is security-sensitive — TDD on every task in `stint-core` is non-negotiable. Two execution options:

**1. Subagent-Driven (recommended)** — Fresh subagent per task with two-stage review (spec + code-quality). Tasks 15 (CLI), 16 (Tauri), 17 (UI), and 19 (manual end-to-end) naturally have human checkpoints because they touch surfaces or external systems.

**2. Inline Execution** — Execute tasks in this session with checkpoint pauses at the end of every Rust task (TDD discipline), before Task 15 (CLI surface — different crate), before Task 16/17 (GUI — needs Tauri context), and at Task 19 (PR + merge + tag).

Either way, **stop and confirm with the human** before:
- Pushing the branch (Task 19 step 2)
- Merging the PR (Task 19 step 5)
- Pushing the tag (Task 19 step 6)
- Any step where the OAuth flow needs to be tested against a real Solidtime instance (Task 19 step 4 — requires server-side configuration the agent can't do)

Which approach?
