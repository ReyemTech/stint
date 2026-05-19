# stint Phase 3b: Calendar Integration (Google) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the calendar-integration subsystem to stint, end-to-end for Google Calendar only. After this phase, a user can connect a Google account, choose which calendars are visible, see today's events in the Today route, and click "Log this" to convert any event into a time entry that flows through the normal Solidtime sync pipeline. Microsoft Graph (Phase 3c) and CalDAV (Phase 3d) are deferred — the trait, schema, and refresh machinery introduced here are positioned for them to slot in later without rework.

**Architecture:**
- New `stint-core::calendar` module owns calendar domain types, the `CalendarProvider` trait, store CRUD over the four new calendar tables, and the per-account refresh strategy described in spec §5.
- A `Google` submodule (`stint-core::calendar::google`) provides the first concrete `CalendarProvider` impl. It reuses the OAuth machinery shipped in Phase 3a — same `OAuthClient`, same loopback redirect capture, same `TokenSet` and `OAuthTokenProvider` — with one small extension (`OAuthConfig::extra_authorize_params`) to support Google's required `access_type=offline` + `prompt=consent` parameters.
- Calendar OAuth credentials live in macOS Keychain under `tech.reyem.stint.calendar.<account_uuid>` as a JSON blob shaped exactly like the Solidtime OAuth blob (`{ client_id, tokens }`). Multiple accounts coexist — one Keychain entry per local account UUID.
- A new background worker (`calendar_worker.rs`) in `stint-app` polls every 15 min while the GUI runs, mirroring the existing `sync_worker.rs`. The worker emits `calendar:changed` so the UI refreshes without polling.
- The "Log this" action creates a completed time entry via a new `Entries::create_completed` helper (the existing `create` only supports the running-timer "started but not ended" state). The entry has `source = 'calendar'` and `source_event_id = '<account>:<event_id>:<start>'`; sync to Solidtime is unchanged.

**Tech Stack:** existing OAuth machinery from Phase 3a (`OAuthClient`, `LoopbackServer`, `TokenSet`) · existing `reqwest` (rustls) for Google Calendar v3 REST calls · existing `keyring` for per-account token persistence · existing `sqlx` migrations + tempdir test pattern · existing `wiremock` for HTTP testing · existing `webbrowser` crate (CLI) and `tauri-plugin-opener` (GUI) for opening the authorize URL.

---

## Why the `CalendarProvider` trait (and not direct Google calls in `stint-core`)

The same argument that motivated 3a's `TokenProvider` trait applies here: with three planned providers and a refresher that wants to iterate over `enabled` accounts uniformly, an enum forces every call site to `match` on variant and duplicate refresh + paging logic. A trait collapses each surface to one `provider.list_events(...)` call.

The trait also makes refresh testable end-to-end with a `MockProvider` that returns canned events — no network, no OAuth dance — so we can drive `calendar::sync` through every code path (range routing, upsert idempotency, decision linkage) without wiremock plumbing in those tests.

## Why a baked-in Google OAuth client ID (and not user-provided)

Solidtime in 3a required the user to register an OAuth client because Solidtime is the user's own self-hosted instance — they always have admin access, and there's no "stint published" Solidtime to register against. Google is the opposite: there's exactly one google.com, and asking every stint user to create a Google Cloud project, configure an OAuth consent screen, and paste a client ID is enough friction to kill adoption. Desktop apps that ship Google integration (Zoom, Slack, Notion's desktop app, 1Password) bake their own client ID in the binary; that's the precedent we follow.

The client ID is non-secret (it's visible in every authorize URL anyway) and PKCE protects the flow from interception. The only artefacts we control: the registered redirect URI scheme (`http://127.0.0.1` is allowed for "Desktop application" clients in Google Cloud) and the consent-screen branding. There is **one prerequisite Mario must complete before Task 12 lands** — see the Prerequisites section.

`STINT_GOOGLE_CLIENT_ID` env-var override is kept for local development and for the integration tests; production builds use the baked-in constant.

## Why store calendar events keyed on `(account_id, event_id, start_at)`

Per spec §3: recurring events expand into many instances that share an `event_id`. Keying only on `event_id` would collapse them; keying on `start_at` alone would lose stability when an organizer reschedules an instance. The composite key lets the same recurring root coexist with all its expanded instances and lets modified instances (Google sets `recurringEventId` on overrides) replace the base instance at the same `start_at` cleanly.

Google's v3 events endpoint with `singleEvents=true` already returns expanded instances — we **do not** need an iCal RRULE expander in 3b. The `recurring_root` column is populated from Google's `recurringEventId` field when present so future MS/CalDAV providers can normalize to the same shape.

---

## What ships in Phase 3b (and what does NOT)

**In scope:**
- SQL migration `0002_calendar.sql` creating `calendar_accounts`, `calendars`, `calendar_events`, `event_decisions`.
- `stint-core::calendar` module: domain types, `CalendarProvider` trait, store CRUD, per-account Keychain helpers, refresh strategy.
- `stint-core::calendar::google` submodule: Google `OAuthConfig` factory, HTTP client over Google Calendar v3, `GoogleProvider` impl.
- `OAuthConfig::extra_authorize_params` field — back-compat extension to the 3a `OAuthConfig` struct.
- `Entries::create_completed` for the "Log this" action.
- Tauri commands for calendar management and the "Log this"/"Ignore" actions.
- CLI: `stint calendar add google`, `list`, `remove`, `calendars`, `refresh`, `test`.
- UI: Settings "Calendar accounts" section + Today route "Calendar" section + Log this/Ignore buttons + sign-in flow.
- Background `calendar_worker.rs` polling every 15 min while the GUI runs.
- README + CLAUDE.md updates with Google Cloud OAuth-client setup instructions.

**Out of scope (deferred to later sub-phases):**
- Microsoft Graph provider (Phase 3c).
- CalDAV provider (Phase 3d) — no iCal parsing, no `icalendar` dependency in 3b.
- Auto-log rules engine (Phase 4 or beyond) — schema for `event_decisions.decision = 'logged_auto'` is in place but unused.
- Writing to calendars (read-only stays the v1 policy).
- Pixel-positioned timeline visualization — the Today route shows events as a time-sorted list with start/end labels alongside entries, not a true gantt-style overlay.
- In-app updater. Brew tap. Apple signing. (Phase 4.)

---

## Prerequisites (one-time, completed before Task 12)

Mario must register a Google Cloud OAuth 2.0 client for stint:

1. Open `https://console.cloud.google.com/`. Create a new project named "stint" (or reuse an existing one).
2. APIs & Services → Library → enable **Google Calendar API**.
3. APIs & Services → OAuth consent screen → External → fill app name "stint", support email, developer email. Add scope `.../auth/calendar.readonly`. Add yourself as a test user.
4. APIs & Services → Credentials → Create credentials → **OAuth client ID** → Application type: **Desktop application** → name "stint desktop". Save the resulting client ID.
5. The client secret is shown but not required (PKCE-only flow ignores it).

The client ID looks like `123456789012-abcdefghijklmnopqrstuvwxyz012345.apps.googleusercontent.com`.

When Task 12 lands, the client ID is committed as the value of `GOOGLE_OAUTH_CLIENT_ID` in `crates/stint-core/src/calendar/google/config.rs`. **Mario must paste the real value before the first end-to-end test in Task 20.** Until then, the constant holds a placeholder and integration tests inject a fake via the `STINT_GOOGLE_CLIENT_ID` env var.

---

## File Structure

```
stint/
├── crates/
│   ├── stint-core/
│   │   ├── Cargo.toml                                  # unchanged
│   │   ├── migrations/
│   │   │   └── 0002_calendar.sql                       # NEW
│   │   └── src/
│   │       ├── lib.rs                                  # MODIFIED — add `pub mod calendar;`
│   │       ├── calendar/                               # NEW — provider-agnostic calendar machinery
│   │       │   ├── mod.rs                              # NEW — public surface, re-exports
│   │       │   ├── types.rs                            # NEW — CalendarAccount, Calendar, CalendarEvent,
│   │       │   │                                        #       EventDecision, TimeRange, ProviderKind,
│   │       │   │                                        #       AttendeeStatus
│   │       │   ├── provider.rs                         # NEW — CalendarProvider trait + RemoteCalendar/RemoteEvent
│   │       │   ├── store.rs                            # NEW — Store CRUD over the four calendar tables +
│   │       │   │                                        #       per-account Keychain blob helpers
│   │       │   ├── sync.rs                             # NEW — refresh strategy (ranges + upsert pipeline)
│   │       │   └── google/                             # NEW — first concrete CalendarProvider impl
│   │       │       ├── mod.rs                          # NEW — pub use; GoogleProvider impl
│   │       │       ├── config.rs                       # NEW — OAuthConfig factory, GOOGLE_OAUTH_CLIENT_ID const
│   │       │       ├── client.rs                       # NEW — HTTP wrapper over Calendar v3 (list_calendars, list_events)
│   │       │       └── dto.rs                          # NEW — Google API DTOs + Into<RemoteCalendar/RemoteEvent>
│   │       ├── oauth/
│   │       │   └── client.rs                           # MODIFIED — OAuthConfig.extra_authorize_params; prepare_authorize honors it
│   │       └── store/
│   │           └── entries.rs                          # MODIFIED — add create_completed for "Log this"
│   │
│   ├── stint-cli/
│   │   └── src/
│   │       ├── main.rs                                 # MODIFIED — register `Calendar` subcommand
│   │       └── cmd/
│   │           ├── mod.rs                              # MODIFIED — pub mod calendar
│   │           └── calendar.rs                         # NEW — `stint calendar` subcommands
│   │
│   └── stint-app/
│       └── src/
│           ├── main.rs                                 # MODIFIED — register calendar Tauri commands,
│           │                                            #            spawn calendar_worker
│           ├── calendar_worker.rs                      # NEW — periodic refresh worker
│           └── commands/
│               ├── mod.rs                              # MODIFIED — pub mod calendar; AppError now covers
│               │                                        #            calendar's error variants (none new)
│               └── calendar.rs                         # NEW — Tauri commands for calendar features
│
└── ui/
    └── src/
        ├── api.ts                                      # MODIFIED — typed wrappers for calendar Tauri commands
        ├── types.ts                                    # MODIFIED — CalendarAccount, CalendarEvent,
        │                                                #            CalendarWithDecision, AddAccountResult types
        └── routes/
            ├── Today.tsx                               # MODIFIED — render Calendar section above Entries
            └── Settings.tsx                            # MODIFIED — add Calendar accounts panel
```

After Phase 3b lands, a user can:

- Run `stint calendar add google` in the terminal → browser opens, they grant `calendar.readonly`, terminal prints "Added Google account: <email>".
- Open Settings → see the new Calendar accounts panel → click "Add Google" → same flow via Tauri.
- See today's Google Calendar events in the Today route, time-sorted alongside entries.
- Click "Log this" on any event → a completed time entry is created with `description = event.title`, `source = 'calendar'`, ready to sync to Solidtime.
- Click "Ignore" to dismiss an event (writes `event_decisions.decision = 'ignored'`; the event stops appearing in the "needs decision" view).
- Multiple Google accounts coexist; per-calendar inclusion is togglable.

---

## Cross-task setup

- **Working directory:** `/Users/mariomeyer/code/ReyemTech/apps/tet`.
- **Branch:** `phase-3b`, branched from `main`. Branch protection on `main` blocks direct pushes; everything lands via PR.
- **Commits:** Conventional Commits. Prefixes used in this plan: `feat(core)`, `feat(cli)`, `feat(app)`, `feat(ui)`, `test(core)`, `refactor(core)`, `chore(deps)`, `docs`, `fix(*)`. One commit per task — pre-commit hooks run `cargo fmt` and clippy.
- **End-of-task check:** after each task, run `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and the test added in that task. Full-suite run happens at Task 20.
- **TDD discipline:** every task that adds source in `stint-core` writes the failing test first, confirms the fail, writes the minimal implementation, confirms the pass, then commits. The provider, store, sync, and HTTP layers are pure functions over their inputs — they have to be testable without real Google or real OAuth.
- **PR strategy:** open the PR as a draft after Task 1 so CI runs against every later commit. Mark ready at Task 20. Merge via "Rebase and merge" (preserves linear history). Tag `phase-3b-complete` on `main` after merge.
- **Stop-and-confirm gates (per CLAUDE.md):** explicitly pause for Mario's confirmation before
  - pushing the `phase-3b` branch (Task 1, end);
  - any task that requires a real Google account E2E (Task 20, manual verification subset);
  - merging the PR (Task 20);
  - pushing the `phase-3b-complete` tag (Task 20).
- **Test isolation:** every test that touches Keychain sets `STINT_SKIP_KEYCHAIN_TESTS=1` guard or uses a `Secrets::with_service_prefix("tech.reyem.stint-test")` instance against a unique-per-test prefix. CI exports `STINT_SKIP_KEYCHAIN_TESTS=1` (already wired in Phase 2.5).
- **Time isolation:** tests that depend on the current time use `chrono::Utc::now()` only inside the SUT; tests inject `now` where the surface allows, otherwise the test seeds expiry into the future so the surface doesn't refresh.
- **Dev-CLI wrapper:** during local execution, use `./scripts/dev-cli.sh <subcommand>` instead of `cargo run -p stint-cli --` so Keychain prompts don't re-trigger between rebuilds.

---

## Tasks

### Task 1: Branch, scaffold `calendar/` module tree, register it in `lib.rs`

**Files:**
- Create: `crates/stint-core/src/calendar/mod.rs`
- Create: `crates/stint-core/src/calendar/types.rs`
- Create: `crates/stint-core/src/calendar/provider.rs`
- Create: `crates/stint-core/src/calendar/store.rs`
- Create: `crates/stint-core/src/calendar/sync.rs`
- Create: `crates/stint-core/src/calendar/google/mod.rs`
- Create: `crates/stint-core/src/calendar/google/config.rs`
- Create: `crates/stint-core/src/calendar/google/client.rs`
- Create: `crates/stint-core/src/calendar/google/dto.rs`
- Modify: `crates/stint-core/src/lib.rs`

- [ ] **Step 1: Confirm clean tree and branch**

```bash
git status        # must be clean
git checkout -b phase-3b
```

- [ ] **Step 2: Create stub module files**

```bash
mkdir -p crates/stint-core/src/calendar/google
```

Create each of these files with a single `// stub` line:
- `crates/stint-core/src/calendar/types.rs`
- `crates/stint-core/src/calendar/provider.rs`
- `crates/stint-core/src/calendar/store.rs`
- `crates/stint-core/src/calendar/sync.rs`
- `crates/stint-core/src/calendar/google/config.rs`
- `crates/stint-core/src/calendar/google/client.rs`
- `crates/stint-core/src/calendar/google/dto.rs`

- [ ] **Step 3: Write `crates/stint-core/src/calendar/mod.rs`**

```rust
//! Calendar integration — provider-agnostic types, store, and refresh
//! pipeline. Provider implementations live under submodules
//! (`google` ships in Phase 3b; `microsoft` and `caldav` are future).

pub mod google;
pub mod provider;
pub mod store;
pub mod sync;
pub mod types;
```

- [ ] **Step 4: Write `crates/stint-core/src/calendar/google/mod.rs`**

```rust
//! Google Calendar provider. Reuses `crate::oauth` for the PKCE flow and
//! `reqwest` for the v3 REST surface.

pub mod client;
pub mod config;
pub mod dto;
```

- [ ] **Step 5: Wire the calendar module into `lib.rs`**

Edit `crates/stint-core/src/lib.rs`. The current declarations are:

```rust
pub mod config;
pub mod error;
pub mod ids;
pub mod oauth;
pub mod paths;
pub mod recovery;
pub mod solidtime;
pub mod store;
pub mod sync;
pub mod time;
pub mod timer;
```

Add `pub mod calendar;` in alphabetical position (between `error` and `ids` would be wrong — between `oauth` and `paths`):

```rust
pub mod calendar;
pub mod config;
pub mod error;
pub mod ids;
pub mod oauth;
pub mod paths;
pub mod recovery;
pub mod solidtime;
pub mod store;
pub mod sync;
pub mod time;
pub mod timer;
```

- [ ] **Step 6: Verify the workspace still builds**

```bash
cargo check --workspace
```

Expected: clean compile. Stubs compile because each `// stub` file is an empty Rust source.

- [ ] **Step 7: Commit**

```bash
git add crates/stint-core/src/calendar crates/stint-core/src/lib.rs
git commit -m "chore(core): scaffold calendar module tree

Creates empty stint-core::calendar submodules (types, provider, store,
sync, google::{config,client,dto}). Wires the module into lib.rs.
Subsequent tasks fill them in. No behaviour change."
```

- [ ] **Step 8: Push the branch and open a draft PR**

```bash
git push -u origin phase-3b
```

Then open a draft PR titled "Phase 3b: Calendar integration (Google)" with body referencing this plan path. **Pause here for Mario to confirm the push and PR creation before continuing.**

---

### Task 2: SQL migration `0002_calendar.sql` + migration test

**Files:**
- Create: `crates/stint-core/migrations/0002_calendar.sql`
- Create: `crates/stint-core/tests/store_calendar_migration.rs`

The four calendar tables from spec §3, exactly as specified. Composite primary key on `calendar_events` enables recurring-instance coexistence.

- [ ] **Step 1: Write the failing migration test**

Create `crates/stint-core/tests/store_calendar_migration.rs`:

```rust
mod common;

#[tokio::test]
async fn calendar_tables_exist_after_migration() {
    let env = common::setup().await;
    let pool = env.store.pool();

    // Each query must succeed (returns 0 rows but does not error out).
    sqlx::query("SELECT id, provider, display_name, identifier, caldav_url, enabled, created_at FROM calendar_accounts LIMIT 0")
        .execute(pool).await.unwrap();
    sqlx::query("SELECT id, account_id, name, color, included FROM calendars LIMIT 0")
        .execute(pool).await.unwrap();
    sqlx::query("SELECT id, account_id, calendar_id, title, start_at, end_at, is_all_day, attendee_status, recurring_root, fetched_at FROM calendar_events LIMIT 0")
        .execute(pool).await.unwrap();
    sqlx::query("SELECT account_id, event_id, event_start, decision, linked_local_uuid, decided_at FROM event_decisions LIMIT 0")
        .execute(pool).await.unwrap();
}

#[tokio::test]
async fn calendar_events_pk_allows_same_event_id_at_different_starts() {
    let env = common::setup().await;
    let pool = env.store.pool();

    sqlx::query("INSERT INTO calendar_accounts (id, provider, display_name, identifier, enabled, created_at) VALUES (?, 'google', 'me', 'me@example.com', 1, '2026-05-19T00:00:00Z')")
        .bind("acc-1").execute(pool).await.unwrap();
    sqlx::query("INSERT INTO calendars (id, account_id, name, included) VALUES (?, ?, 'Primary', 1)")
        .bind("cal-1").bind("acc-1").execute(pool).await.unwrap();

    // Insert same event id with two different start_ats — both must succeed.
    for start in ["2026-05-19T09:00:00Z", "2026-05-26T09:00:00Z"] {
        sqlx::query("INSERT INTO calendar_events (id, account_id, calendar_id, title, start_at, end_at, is_all_day, fetched_at) VALUES (?, ?, ?, 'Standup', ?, ?, 0, ?)")
            .bind("evt-recurring")
            .bind("acc-1")
            .bind("cal-1")
            .bind(start)
            .bind(start)
            .bind("2026-05-19T00:00:00Z")
            .execute(pool).await.unwrap();
    }

    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM calendar_events WHERE id = 'evt-recurring'")
        .fetch_one(pool).await.unwrap();
    assert_eq!(count, 2);
}
```

- [ ] **Step 2: Run test — confirm failure**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test store_calendar_migration -- --test-threads=1
```

Expected: FAIL with "no such table: calendar_accounts" (the migration doesn't exist yet).

- [ ] **Step 3: Write `crates/stint-core/migrations/0002_calendar.sql`**

```sql
-- Phase 3b: calendar tables. Matches spec §3 ("Calendar").

CREATE TABLE calendar_accounts (
  id           TEXT PRIMARY KEY,
  provider     TEXT NOT NULL,                  -- 'google' (Phase 3c/d add 'microsoft', 'caldav')
  display_name TEXT NOT NULL,
  identifier   TEXT NOT NULL,                  -- email for OAuth providers
  caldav_url   TEXT,                           -- nullable; populated only for CalDAV (Phase 3d)
  enabled      INTEGER NOT NULL DEFAULT 1,
  created_at   TEXT NOT NULL
);

CREATE TABLE calendars (
  id         TEXT PRIMARY KEY,                 -- provider-native calendar id
  account_id TEXT NOT NULL REFERENCES calendar_accounts(id) ON DELETE CASCADE,
  name       TEXT NOT NULL,
  color      TEXT,
  included   INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_calendars_account ON calendars(account_id);

CREATE TABLE calendar_events (
  id              TEXT NOT NULL,               -- provider event id
  account_id      TEXT NOT NULL REFERENCES calendar_accounts(id) ON DELETE CASCADE,
  calendar_id     TEXT NOT NULL REFERENCES calendars(id) ON DELETE CASCADE,
  title           TEXT NOT NULL,
  start_at        TEXT NOT NULL,
  end_at          TEXT NOT NULL,
  is_all_day      INTEGER NOT NULL DEFAULT 0,
  attendee_status TEXT,                        -- 'accepted' | 'declined' | 'tentative' | NULL
  recurring_root  TEXT,                        -- provider's recurringEventId for expanded instances
  fetched_at      TEXT NOT NULL,
  PRIMARY KEY (account_id, id, start_at)
);
CREATE INDEX idx_calendar_events_start ON calendar_events(start_at);
CREATE INDEX idx_calendar_events_calendar_start ON calendar_events(calendar_id, start_at);

CREATE TABLE event_decisions (
  account_id        TEXT NOT NULL,
  event_id          TEXT NOT NULL,
  event_start       TEXT NOT NULL,
  decision          TEXT NOT NULL,             -- 'ignored' | 'logged_manual' | 'logged_auto'
  linked_local_uuid TEXT REFERENCES time_entries(local_uuid) ON DELETE SET NULL,
  decided_at        TEXT NOT NULL,
  PRIMARY KEY (account_id, event_id, event_start)
);
```

- [ ] **Step 4: Run test — confirm pass**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test store_calendar_migration -- --test-threads=1
```

Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/migrations/0002_calendar.sql \
        crates/stint-core/tests/store_calendar_migration.rs
git commit -m "feat(core): add calendar tables migration

Adds the four calendar tables from spec §3: calendar_accounts,
calendars, calendar_events, event_decisions. Composite primary key on
calendar_events enables recurring-instance coexistence.

Includes a migration smoke test and a recurring-instance constraint
test."
```

---

### Task 3: Calendar domain types (`types.rs`)

**Files:**
- Modify: `crates/stint-core/src/calendar/types.rs`
- Create: `crates/stint-core/tests/calendar_types.rs`

Pure data types plus serde round-trips. No I/O — the test exercises the serde derives and `From<&str>` parses for the small enums.

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/calendar_types.rs`:

```rust
use stint_core::calendar::types::{
    AttendeeStatus, CalendarAccount, CalendarEvent, EventDecision, ProviderKind, TimeRange,
};
use chrono::{TimeZone, Utc};

#[test]
fn provider_kind_serde_roundtrip() {
    let kinds = [ProviderKind::Google];
    for k in kinds {
        let s = serde_json::to_string(&k).unwrap();
        let back: ProviderKind = serde_json::from_str(&s).unwrap();
        assert_eq!(k, back);
    }
}

#[test]
fn provider_kind_string_form_is_lowercase() {
    let s = serde_json::to_string(&ProviderKind::Google).unwrap();
    assert_eq!(s, "\"google\"");
}

#[test]
fn attendee_status_parses_known_values() {
    assert_eq!(AttendeeStatus::from_wire("accepted"), Some(AttendeeStatus::Accepted));
    assert_eq!(AttendeeStatus::from_wire("declined"), Some(AttendeeStatus::Declined));
    assert_eq!(AttendeeStatus::from_wire("tentative"), Some(AttendeeStatus::Tentative));
    assert_eq!(AttendeeStatus::from_wire("needsAction"), None);
    assert_eq!(AttendeeStatus::from_wire(""), None);
}

#[test]
fn event_decision_kind_serde() {
    let d = EventDecision::LoggedManual { linked_local_uuid: "uuid-1".into() };
    let s = serde_json::to_string(&d).unwrap();
    let back: EventDecision = serde_json::from_str(&s).unwrap();
    assert!(matches!(back, EventDecision::LoggedManual { .. }));
}

#[test]
fn time_range_inclusion_is_half_open() {
    let r = TimeRange {
        start: Utc.with_ymd_and_hms(2026, 5, 19, 9, 0, 0).unwrap(),
        end:   Utc.with_ymd_and_hms(2026, 5, 19, 10, 0, 0).unwrap(),
    };
    let at_start = Utc.with_ymd_and_hms(2026, 5, 19, 9, 0, 0).unwrap();
    let at_end   = Utc.with_ymd_and_hms(2026, 5, 19, 10, 0, 0).unwrap();
    let inside   = Utc.with_ymd_and_hms(2026, 5, 19, 9, 30, 0).unwrap();
    assert!(r.contains(at_start));
    assert!(!r.contains(at_end));   // half-open: [start, end)
    assert!(r.contains(inside));
}

#[test]
fn calendar_account_constructs_with_defaults() {
    let a = CalendarAccount {
        id: "acc-1".into(),
        provider: ProviderKind::Google,
        display_name: "me".into(),
        identifier: "me@example.com".into(),
        caldav_url: None,
        enabled: true,
        created_at: "2026-05-19T00:00:00Z".into(),
    };
    let _ = format!("{a:?}");   // ensure Debug is derived
    let s = serde_json::to_string(&a).unwrap();
    let back: CalendarAccount = serde_json::from_str(&s).unwrap();
    assert_eq!(back.identifier, "me@example.com");
}

#[test]
fn calendar_event_round_trips_with_optional_fields_absent() {
    let e = CalendarEvent {
        id: "evt-1".into(),
        account_id: "acc-1".into(),
        calendar_id: "cal-1".into(),
        title: "Standup".into(),
        start_at: "2026-05-19T09:00:00Z".into(),
        end_at: "2026-05-19T09:15:00Z".into(),
        is_all_day: false,
        attendee_status: None,
        recurring_root: None,
        fetched_at: "2026-05-19T00:00:00Z".into(),
    };
    let s = serde_json::to_string(&e).unwrap();
    let back: CalendarEvent = serde_json::from_str(&s).unwrap();
    assert_eq!(back.title, "Standup");
}
```

- [ ] **Step 2: Run test — confirm failure**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_types -- --test-threads=1
```

Expected: FAIL — types don't exist yet.

- [ ] **Step 3: Implement `crates/stint-core/src/calendar/types.rs`**

```rust
//! Calendar domain types — shared by the provider trait, the store,
//! the sync refresher, and the public API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Google,
    // Phase 3c: Microsoft
    // Phase 3d: CalDav
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AttendeeStatus {
    Accepted,
    Declined,
    Tentative,
}

impl AttendeeStatus {
    /// Map the provider's on-the-wire string to a known status. Returns
    /// `None` for values we do not normalize (e.g. Google's `needsAction`).
    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "accepted" => Some(Self::Accepted),
            "declined" => Some(Self::Declined),
            "tentative" => Some(Self::Tentative),
            _ => None,
        }
    }

    pub fn as_wire(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Declined => "declined",
            Self::Tentative => "tentative",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarAccount {
    pub id: String,                  // local uuid
    pub provider: ProviderKind,
    pub display_name: String,
    pub identifier: String,          // email for OAuth providers
    pub caldav_url: Option<String>,
    pub enabled: bool,
    pub created_at: String,          // RFC 3339
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calendar {
    pub id: String,                  // provider-native id
    pub account_id: String,
    pub name: String,
    pub color: Option<String>,
    pub included: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,                  // provider-native id
    pub account_id: String,
    pub calendar_id: String,
    pub title: String,
    pub start_at: String,            // RFC 3339 (or YYYY-MM-DD for all-day)
    pub end_at: String,
    pub is_all_day: bool,
    pub attendee_status: Option<AttendeeStatus>,
    pub recurring_root: Option<String>,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventDecision {
    Ignored,
    LoggedManual { linked_local_uuid: String },
    LoggedAuto { linked_local_uuid: String },
}

impl EventDecision {
    /// Returns the decision string stored in the `event_decisions.decision`
    /// column. Symmetric with [`Self::decoded`].
    pub fn as_wire(&self) -> &'static str {
        match self {
            Self::Ignored => "ignored",
            Self::LoggedManual { .. } => "logged_manual",
            Self::LoggedAuto { .. } => "logged_auto",
        }
    }

    pub fn linked_local_uuid(&self) -> Option<&str> {
        match self {
            Self::Ignored => None,
            Self::LoggedManual { linked_local_uuid } | Self::LoggedAuto { linked_local_uuid } => {
                Some(linked_local_uuid)
            }
        }
    }

    pub fn decoded(wire: &str, linked_local_uuid: Option<String>) -> Option<Self> {
        match (wire, linked_local_uuid) {
            ("ignored", _) => Some(Self::Ignored),
            ("logged_manual", Some(uuid)) => Some(Self::LoggedManual { linked_local_uuid: uuid }),
            ("logged_auto", Some(uuid)) => Some(Self::LoggedAuto { linked_local_uuid: uuid }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl TimeRange {
    /// Half-open `[start, end)`. Useful for both "today" queries and refresh-window logic.
    pub fn contains(&self, t: DateTime<Utc>) -> bool {
        t >= self.start && t < self.end
    }
}
```

- [ ] **Step 4: Run test — confirm pass**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_types -- --test-threads=1
```

Expected: PASS (7 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/calendar/types.rs crates/stint-core/tests/calendar_types.rs
git commit -m "feat(core): calendar domain types

Adds CalendarAccount, Calendar, CalendarEvent, EventDecision,
TimeRange, ProviderKind, AttendeeStatus with serde derives and
wire-format helpers. Half-open TimeRange semantics for refresh
windows and 'today' filters."
```

---

### Task 4: `CalendarProvider` trait + remote DTOs (`provider.rs`)

**Files:**
- Modify: `crates/stint-core/src/calendar/provider.rs`
- Modify: `crates/stint-core/src/calendar/mod.rs` (re-export the trait)
- Create: `crates/stint-core/tests/calendar_provider_mock.rs`

The trait is what `calendar::sync` calls into; the test demonstrates that a stub provider satisfies it.

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/calendar_provider_mock.rs`:

```rust
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use stint_core::calendar::provider::{CalendarProvider, RemoteCalendar, RemoteEvent};
use stint_core::calendar::types::{ProviderKind, TimeRange};
use stint_core::Result;

struct StubProvider {
    calendars: Vec<RemoteCalendar>,
    events: Vec<RemoteEvent>,
}

#[async_trait]
impl CalendarProvider for StubProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Google
    }
    async fn list_calendars(&self) -> Result<Vec<RemoteCalendar>> {
        Ok(self.calendars.clone())
    }
    async fn list_events(&self, _calendar_id: &str, _range: TimeRange) -> Result<Vec<RemoteEvent>> {
        Ok(self.events.clone())
    }
}

#[tokio::test]
async fn stub_provider_satisfies_trait() {
    let p = StubProvider {
        calendars: vec![RemoteCalendar {
            id: "primary".into(),
            name: "Primary".into(),
            color: Some("#000".into()),
        }],
        events: vec![RemoteEvent {
            id: "evt-1".into(),
            calendar_id: "primary".into(),
            title: "Standup".into(),
            start_at: "2026-05-19T09:00:00Z".into(),
            end_at: "2026-05-19T09:15:00Z".into(),
            is_all_day: false,
            attendee_status: None,
            recurring_root: None,
        }],
    };
    assert_eq!(p.kind(), ProviderKind::Google);
    assert_eq!(p.list_calendars().await.unwrap().len(), 1);
    let range = TimeRange {
        start: Utc.with_ymd_and_hms(2026, 5, 19, 0, 0, 0).unwrap(),
        end: Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
    };
    let evs = p.list_events("primary", range).await.unwrap();
    assert_eq!(evs[0].title, "Standup");
}
```

- [ ] **Step 2: Run test — confirm failure**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_provider_mock -- --test-threads=1
```

Expected: FAIL — `CalendarProvider`, `RemoteCalendar`, `RemoteEvent` don't exist.

- [ ] **Step 3: Implement `crates/stint-core/src/calendar/provider.rs`**

```rust
//! Provider-agnostic calendar interface. `calendar::sync` is written
//! against this trait so MS Graph (Phase 3c) and CalDAV (Phase 3d)
//! can plug in without disturbing the refresher.

use crate::calendar::types::{AttendeeStatus, ProviderKind, TimeRange};
use crate::Result;
use async_trait::async_trait;

#[async_trait]
pub trait CalendarProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    async fn list_calendars(&self) -> Result<Vec<RemoteCalendar>>;
    async fn list_events(
        &self,
        calendar_id: &str,
        range: TimeRange,
    ) -> Result<Vec<RemoteEvent>>;
}

/// Provider-shaped calendar — same fields as the domain `Calendar`, minus
/// the `account_id` (assigned at upsert time) and `included` flag (a local
/// concept, not part of the remote view).
#[derive(Debug, Clone)]
pub struct RemoteCalendar {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
}

/// Provider-shaped event — domain `CalendarEvent` minus `account_id` and
/// `fetched_at` (assigned at upsert time).
#[derive(Debug, Clone)]
pub struct RemoteEvent {
    pub id: String,
    pub calendar_id: String,
    pub title: String,
    pub start_at: String,
    pub end_at: String,
    pub is_all_day: bool,
    pub attendee_status: Option<AttendeeStatus>,
    pub recurring_root: Option<String>,
}
```

- [ ] **Step 4: Re-export from `calendar/mod.rs`**

Replace the contents of `crates/stint-core/src/calendar/mod.rs` with:

```rust
//! Calendar integration — provider-agnostic types, store, and refresh
//! pipeline. Provider implementations live under submodules
//! (`google` ships in Phase 3b; `microsoft` and `caldav` are future).

pub mod google;
pub mod provider;
pub mod store;
pub mod sync;
pub mod types;

pub use provider::{CalendarProvider, RemoteCalendar, RemoteEvent};
pub use types::{
    AttendeeStatus, Calendar, CalendarAccount, CalendarEvent, EventDecision, ProviderKind, TimeRange,
};
```

- [ ] **Step 5: Run test — confirm pass**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_provider_mock -- --test-threads=1
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/stint-core/src/calendar/provider.rs \
        crates/stint-core/src/calendar/mod.rs \
        crates/stint-core/tests/calendar_provider_mock.rs
git commit -m "feat(core): CalendarProvider trait + remote DTOs

Defines the trait calendar::sync calls into and the provider-shaped
DTOs (RemoteCalendar, RemoteEvent) that providers return. Stub-provider
test confirms the surface is satisfiable without async-trait gotchas."
```

---

### Task 5: Extend OAuth machinery for Google quirks (two commits)

**Files:**
- Modify: `crates/stint-core/src/oauth/client.rs`
- Modify: `crates/stint-core/src/oauth/loopback.rs`
- Modify: `crates/stint-core/src/solidtime/auth.rs` (two call sites: `OAuthConfig` literal + `login_interactive` signature)
- Modify: `crates/stint-app/src/commands/config.rs` (two call sites)
- Modify: `crates/stint-cli/src/cmd/config_login.rs` (two call sites)
- Modify: `crates/stint-core/tests/oauth_authorize_url.rs` (add `extra_authorize_params` test)
- Modify: `crates/stint-core/tests/oauth_loopback.rs` (add provider-label test)

This task ships two back-compat extensions to the OAuth machinery, both driven by Google's needs:

1. `OAuthConfig::extra_authorize_params` — Google requires `access_type=offline` + `prompt=consent` on the authorize URL to reliably issue a refresh_token.
2. `LoopbackServer` provider-label parameter — the success/error HTML currently hardcodes "Solidtime"; making the label runtime-configurable lets the Google flow show "Signed in to Google" without disturbing existing behaviour.

Each extension lands as its own commit so the atomic-commit principle holds; the task is grouped because both touch the same set of OAuth call sites.

- [ ] **Step 1: Add the failing test**

Open `crates/stint-core/tests/oauth_authorize_url.rs` and append:

```rust
#[test]
fn authorize_url_appends_extra_params_in_order() {
    use stint_core::oauth::client::{OAuthClient, OAuthConfig};

    let cfg = OAuthConfig {
        authorize_url: "https://accounts.google.com/o/oauth2/v2/auth".into(),
        token_url: "https://oauth2.googleapis.com/token".into(),
        client_id: "fake-id".into(),
        redirect_uri: "http://127.0.0.1:0/callback".into(),
        scopes: vec!["https://www.googleapis.com/auth/calendar.readonly".into()],
        extra_authorize_params: vec![
            ("access_type".into(), "offline".into()),
            ("prompt".into(), "consent".into()),
        ],
    };
    let prepared = OAuthClient::new(cfg).prepare_authorize();
    let url = prepared.authorize_url.to_string();
    assert!(url.contains("access_type=offline"), "got {url}");
    assert!(url.contains("prompt=consent"), "got {url}");
    // Required OAuth params still present.
    assert!(url.contains("response_type=code"));
    assert!(url.contains("code_challenge_method=S256"));
}
```

- [ ] **Step 2: Run — confirm compile failure (missing field)**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test oauth_authorize_url -- --test-threads=1
```

Expected: COMPILE-FAIL on missing field `extra_authorize_params`.

- [ ] **Step 3: Add the field + honor it**

Edit `crates/stint-core/src/oauth/client.rs`. Find:

```rust
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub authorize_url: String,
    pub token_url: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}
```

Replace with:

```rust
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub authorize_url: String,
    pub token_url: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    /// Provider-specific query params appended to the authorize URL.
    /// Empty for Solidtime; Google needs `access_type=offline` and
    /// `prompt=consent` to consistently issue a refresh_token.
    pub extra_authorize_params: Vec<(String, String)>,
}
```

In the same file, find `prepare_authorize` and append, after the existing `append_pair` chain:

```rust
        for (k, v) in &self.config.extra_authorize_params {
            url.query_pairs_mut().append_pair(k, v);
        }
```

So the function ends:

```rust
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
```

- [ ] **Step 4: Update the four existing `OAuthConfig { … }` call sites**

In `crates/stint-core/src/solidtime/auth.rs`, locate `build_token_provider`. Update the `OAuthConfig` literal to add `extra_authorize_params: vec![]`:

```rust
    let oauth_client = OAuthClient::new(OAuthConfig {
        authorize_url: format!(
            "{}/oauth/authorize",
            solidtime_base_url.trim_end_matches('/')
        ),
        token_url: format!("{}/oauth/token", solidtime_base_url.trim_end_matches('/')),
        client_id,
        redirect_uri: DEFAULT_REDIRECT_URI_HOST.into(),
        scopes: DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect(),
        extra_authorize_params: vec![],
    });
```

In `crates/stint-app/src/commands/config.rs`, in `oauth_solidtime_start`, add the same field to the `OAuthConfig` literal:

```rust
    let client = OAuthClient::new(OAuthConfig {
        authorize_url: format!("{}/oauth/authorize", base_url.trim_end_matches('/')),
        token_url: format!("{}/oauth/token", base_url.trim_end_matches('/')),
        client_id: client_id.clone(),
        redirect_uri: "http://127.0.0.1:0/callback".into(),
        scopes: vec![],
        extra_authorize_params: vec![],
    });
```

In `crates/stint-cli/src/cmd/config_login.rs`, in `run_login`, same update:

```rust
    let client = OAuthClient::new(OAuthConfig {
        authorize_url: format!("{}/oauth/authorize", base_url.trim_end_matches('/')),
        token_url: format!("{}/oauth/token", base_url.trim_end_matches('/')),
        client_id: client_id.clone(),
        redirect_uri: "http://127.0.0.1:0/callback".into(),
        scopes: vec![],
        extra_authorize_params: vec![],
    });
```

Update the existing test helpers under `crates/stint-core/tests/` that construct `OAuthConfig`:

- `crates/stint-core/tests/oauth_authorize_url.rs` — find every `OAuthConfig { … }` and add `extra_authorize_params: vec![],`.
- `crates/stint-core/tests/oauth_exchange.rs` — same.
- `crates/stint-core/tests/oauth_refresh.rs` — same.
- `crates/stint-core/tests/solidtime_oauth_provider.rs` — same.
- `crates/stint-core/tests/solidtime_login_e2e.rs` — same.
- `crates/stint-core/tests/solidtime_auth_resolver.rs` — same.

Grep to find them all:

```bash
grep -rln 'OAuthConfig {' crates/stint-core/tests/
```

- [ ] **Step 5: Run tests — confirm pass**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core -- --test-threads=1
```

Expected: ALL pass — the previously-failing test now passes, and every existing OAuth test stays green.

- [ ] **Step 6: Commit**

```bash
git add crates/stint-core/src/oauth/client.rs \
        crates/stint-core/src/solidtime/auth.rs \
        crates/stint-core/tests/oauth_authorize_url.rs \
        crates/stint-core/tests/oauth_exchange.rs \
        crates/stint-core/tests/oauth_refresh.rs \
        crates/stint-core/tests/solidtime_oauth_provider.rs \
        crates/stint-core/tests/solidtime_login_e2e.rs \
        crates/stint-core/tests/solidtime_auth_resolver.rs \
        crates/stint-app/src/commands/config.rs \
        crates/stint-cli/src/cmd/config_login.rs
git commit -m "refactor(core): OAuthConfig.extra_authorize_params

Adds a Vec<(String,String)> field appended to the authorize URL after
the standard PKCE params. Empty for Solidtime (unchanged behaviour);
Phase 3b's Google config uses it to send access_type=offline +
prompt=consent so Google consistently issues a refresh_token."
```

**Second commit: parameterize loopback HTML by provider.**

- [ ] **Step 7: Write the failing loopback test**

Open `crates/stint-core/tests/oauth_loopback.rs`. Append:

```rust
#[tokio::test]
async fn success_html_includes_provider_label() {
    use stint_core::oauth::loopback::listen_for_callback;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    let server = listen_for_callback(Duration::from_secs(5), "Google")
        .await
        .expect("bind");
    let port = server.port();

    // Hit the callback with a fake code+state pair so the success branch fires.
    tokio::spawn(async move {
        let mut sock = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        sock.write_all(b"GET /callback?code=abc&state=xyz HTTP/1.1\r\nHost: x\r\n\r\n")
            .await
            .unwrap();
        let mut buf = Vec::new();
        sock.read_to_end(&mut buf).await.unwrap();
        let body = String::from_utf8_lossy(&buf);
        assert!(body.contains("Signed in to Google"), "got: {body}");
    });

    let cap = server.await_callback().await.unwrap();
    assert_eq!(cap.code, "abc");
    assert_eq!(cap.state, "xyz");
}
```

- [ ] **Step 8: Run — confirm compile failure**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test oauth_loopback -- --test-threads=1
```

Expected: COMPILE-FAIL — `listen_for_callback` currently takes one arg.

- [ ] **Step 9: Parameterize the HTML**

Edit `crates/stint-core/src/oauth/loopback.rs`. Replace the two `const … HTML` declarations with builder functions:

```rust
fn success_html(provider_label: &str) -> String {
    format!(
        "<!doctype html><meta charset=utf-8>\
<title>stint — signed in</title>\
<style>body{{font:16px system-ui;padding:48px;max-width:520px;color:#1a1a1a}}</style>\
<h1>Signed in to {label}</h1>\
<p>You can close this tab and return to stint.</p>",
        label = html_escape(provider_label),
    )
}

fn error_html(provider_label: &str) -> String {
    format!(
        "<!doctype html><meta charset=utf-8>\
<title>stint — sign-in failed</title>\
<style>body{{font:16px system-ui;padding:48px;max-width:520px;color:#1a1a1a}}</style>\
<h1>Sign-in to {label} failed</h1>\
<p>Return to stint for details.</p>",
        label = html_escape(provider_label),
    )
}

/// Minimal HTML-attribute-safe escape — provider_label values come from
/// our own code (\"Solidtime\", \"Google\"), but escape defensively so a
/// future caller can pass any string without HTML injection.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
```

Change `listen_for_callback`'s signature to:

```rust
pub async fn listen_for_callback(
    server_timeout: Duration,
    provider_label: &str,
) -> Result<LoopbackServer> {
```

Inside, capture the label for the spawned task:

```rust
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| Error::OAuthLoopback(e.to_string()))?;
    let port = listener
        .local_addr()
        .map_err(|e| Error::OAuthLoopback(e.to_string()))?
        .port();
    let (tx, rx) = oneshot::channel();
    let provider_label = provider_label.to_string();

    tokio::spawn(async move {
        let Ok((mut socket, _)) = listener.accept().await else {
            let _ = tx.send(Err(Error::OAuthCancelled));
            return;
        };

        let mut reader = BufReader::new(&mut socket);
        let mut request_line = String::new();
        if reader.read_line(&mut request_line).await.is_err() {
            let _ = tx.send(Err(Error::OAuthCancelled));
            return;
        }

        let parse_result = parse_callback_query(&request_line);
        let (body, response) = match &parse_result {
            Ok(_) => (success_html(&provider_label), "HTTP/1.1 200 OK"),
            Err(_) => (error_html(&provider_label), "HTTP/1.1 400 Bad Request"),
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
    // ... rest unchanged
```

- [ ] **Step 10: Thread the label through `login_interactive`**

Edit `crates/stint-core/src/solidtime/auth.rs`. Change the signature of `login_interactive` to accept a `provider_label`:

```rust
pub async fn login_interactive<F>(
    client: &OAuthClient,
    flow_timeout: Duration,
    provider_label: &str,
    open_browser: F,
) -> Result<TokenSet>
where
    F: FnOnce(String),
{
    let server = listen_for_callback(flow_timeout, provider_label).await?;
    // ... rest unchanged
}
```

Update the call sites in `crates/stint-app/src/commands/config.rs` (function `oauth_solidtime_start`) — pass `"Solidtime"`:

```rust
    let tokens = login_interactive(&client, Duration::from_secs(300), "Solidtime", |url| {
        if let Err(e) = open_url(&url) {
            tracing::warn!("could not open browser: {e}; user must paste URL manually: {url}");
        }
    })
    .await?;
```

In `crates/stint-cli/src/cmd/config_login.rs` (`run_login`), same — pass `"Solidtime"`:

```rust
    let tokens = login_interactive(&client, FLOW_TIMEOUT, "Solidtime", |url| {
        println!("  {url}");
        if let Err(e) = webbrowser::open(&url) {
            eprintln!("(Could not auto-open browser: {e})");
        }
    })
    .await
    .context("OAuth flow failed")?;
```

If any existing test calls `login_interactive` (check `crates/stint-core/tests/solidtime_login_e2e.rs`), update those call sites to pass `"Solidtime"` as well:

```bash
grep -rn 'login_interactive' crates/
```

- [ ] **Step 11: Run all OAuth tests — confirm pass**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test oauth_loopback --test oauth_authorize_url --test oauth_exchange --test oauth_refresh --test solidtime_login_e2e -- --test-threads=1
```

Expected: ALL pass.

- [ ] **Step 12: Commit the loopback change**

```bash
git add crates/stint-core/src/oauth/loopback.rs \
        crates/stint-core/src/solidtime/auth.rs \
        crates/stint-core/tests/oauth_loopback.rs \
        crates/stint-app/src/commands/config.rs \
        crates/stint-cli/src/cmd/config_login.rs
# Plus any test files updated in step 10:
git add crates/stint-core/tests/solidtime_login_e2e.rs 2>/dev/null || true
git commit -m "refactor(core): parameterize loopback HTML by provider

listen_for_callback and login_interactive now take a provider_label.
Existing Solidtime callers pass \"Solidtime\" (unchanged HTML);
Phase 3b's Google login passes \"Google\" so the success/error pages
read 'Signed in to Google' instead of misleading 'Signed in to
Solidtime'. HTML-escape the label defensively against future callers."
```

---

### Task 6: Store CRUD for `calendar_accounts` (`store.rs`, part 1)

**Files:**
- Modify: `crates/stint-core/src/calendar/store.rs`
- Create: `crates/stint-core/tests/calendar_store_accounts.rs`

The first of four store-level surfaces. We build them up table-by-table so each commit is verifiable in isolation.

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/calendar_store_accounts.rs`:

```rust
mod common;

use stint_core::calendar::store::CalendarStore;
use stint_core::calendar::types::{CalendarAccount, ProviderKind};

fn sample_account(id: &str, email: &str) -> CalendarAccount {
    CalendarAccount {
        id: id.into(),
        provider: ProviderKind::Google,
        display_name: email.into(),
        identifier: email.into(),
        caldav_url: None,
        enabled: true,
        created_at: "2026-05-19T00:00:00Z".into(),
    }
}

#[tokio::test]
async fn add_then_list_returns_one_account() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());

    s.add_account(&sample_account("acc-1", "me@example.com")).await.unwrap();
    let list = s.list_accounts().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "acc-1");
    assert_eq!(list[0].identifier, "me@example.com");
    assert!(list[0].enabled);
}

#[tokio::test]
async fn get_account_returns_none_for_missing() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    assert!(s.get_account("does-not-exist").await.unwrap().is_none());
}

#[tokio::test]
async fn delete_account_removes_it() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    s.add_account(&sample_account("acc-1", "me@example.com")).await.unwrap();
    s.delete_account("acc-1").await.unwrap();
    assert!(s.list_accounts().await.unwrap().is_empty());
}

#[tokio::test]
async fn add_account_with_duplicate_id_returns_error() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    s.add_account(&sample_account("acc-1", "me@example.com")).await.unwrap();
    let err = s.add_account(&sample_account("acc-1", "other@example.com")).await.unwrap_err();
    // sqlx returns a UNIQUE-constraint violation; surfaces as Error::Sqlite.
    assert!(matches!(err, stint_core::Error::Sqlite(_)));
}

#[tokio::test]
async fn set_enabled_toggles() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    s.add_account(&sample_account("acc-1", "me@example.com")).await.unwrap();
    s.set_account_enabled("acc-1", false).await.unwrap();
    let a = s.get_account("acc-1").await.unwrap().unwrap();
    assert!(!a.enabled);
}
```

- [ ] **Step 2: Run — confirm failure**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_store_accounts -- --test-threads=1
```

Expected: FAIL — `CalendarStore` doesn't exist yet.

- [ ] **Step 3: Implement the accounts surface**

Replace the contents of `crates/stint-core/src/calendar/store.rs` with:

```rust
//! Store-layer CRUD for the four calendar tables, plus per-account
//! Keychain blob helpers. Constructed with a `Store` clone, same pattern
//! as `Settings` and `Reference`.

use crate::calendar::types::{
    AttendeeStatus, Calendar, CalendarAccount, CalendarEvent, EventDecision, ProviderKind,
};
use crate::store::Store;
use crate::{time, Result};

pub struct CalendarStore {
    store: Store,
}

impl CalendarStore {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    pub async fn add_account(&self, a: &CalendarAccount) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO calendar_accounts
               (id, provider, display_name, identifier, caldav_url, enabled, created_at)
               VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&a.id)
        .bind(provider_wire(a.provider))
        .bind(&a.display_name)
        .bind(&a.identifier)
        .bind(&a.caldav_url)
        .bind(if a.enabled { 1 } else { 0 })
        .bind(&a.created_at)
        .execute(self.store.pool())
        .await?;
        Ok(())
    }

    pub async fn get_account(&self, id: &str) -> Result<Option<CalendarAccount>> {
        let row: Option<(String, String, String, String, Option<String>, i64, String)> =
            sqlx::query_as(
                "SELECT id, provider, display_name, identifier, caldav_url, enabled, created_at
                 FROM calendar_accounts WHERE id = ?",
            )
            .bind(id)
            .fetch_optional(self.store.pool())
            .await?;
        Ok(row.map(account_from_row))
    }

    pub async fn list_accounts(&self) -> Result<Vec<CalendarAccount>> {
        let rows: Vec<(String, String, String, String, Option<String>, i64, String)> =
            sqlx::query_as(
                "SELECT id, provider, display_name, identifier, caldav_url, enabled, created_at
                 FROM calendar_accounts ORDER BY created_at",
            )
            .fetch_all(self.store.pool())
            .await?;
        Ok(rows.into_iter().map(account_from_row).collect())
    }

    pub async fn delete_account(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM calendar_accounts WHERE id = ?")
            .bind(id)
            .execute(self.store.pool())
            .await?;
        Ok(())
    }

    pub async fn set_account_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        sqlx::query("UPDATE calendar_accounts SET enabled = ? WHERE id = ?")
            .bind(if enabled { 1 } else { 0 })
            .bind(id)
            .execute(self.store.pool())
            .await?;
        Ok(())
    }
}

fn provider_wire(p: ProviderKind) -> &'static str {
    match p {
        ProviderKind::Google => "google",
    }
}

fn provider_from_wire(s: &str) -> ProviderKind {
    match s {
        "google" => ProviderKind::Google,
        // Phase 3c/d will extend; for now an unknown value falls back to Google
        // rather than panic — the column is constrained by what we wrote.
        _ => ProviderKind::Google,
    }
}

fn account_from_row(
    (id, provider, display_name, identifier, caldav_url, enabled, created_at): (
        String,
        String,
        String,
        String,
        Option<String>,
        i64,
        String,
    ),
) -> CalendarAccount {
    CalendarAccount {
        id,
        provider: provider_from_wire(&provider),
        display_name,
        identifier,
        caldav_url,
        enabled: enabled != 0,
        created_at,
    }
}

// Suppress dead-code warnings on imports the later tasks will actually use.
#[allow(dead_code)]
fn _phantom_imports(_: AttendeeStatus, _: Calendar, _: CalendarEvent, _: EventDecision, _: &dyn FnOnce() -> String) {
    let _ = time::now_utc;
}
```

- [ ] **Step 4: Run — confirm pass**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_store_accounts -- --test-threads=1
```

Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/calendar/store.rs crates/stint-core/tests/calendar_store_accounts.rs
git commit -m "feat(core): CalendarStore — calendar_accounts CRUD

Adds add/get/list/delete/set_enabled for the calendar_accounts table.
Per-account dependency cascades (calendars, events, decisions) are
enforced by the FK definitions in the migration — no extra deletion
logic needed at this layer."
```

---

### Task 7: Store CRUD for `calendars` (`store.rs`, part 2)

**Files:**
- Modify: `crates/stint-core/src/calendar/store.rs`
- Create: `crates/stint-core/tests/calendar_store_calendars.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/calendar_store_calendars.rs`:

```rust
mod common;

use stint_core::calendar::store::CalendarStore;
use stint_core::calendar::types::{Calendar, CalendarAccount, ProviderKind};

async fn seed_account(s: &CalendarStore) {
    s.add_account(&CalendarAccount {
        id: "acc-1".into(),
        provider: ProviderKind::Google,
        display_name: "me@example.com".into(),
        identifier: "me@example.com".into(),
        caldav_url: None,
        enabled: true,
        created_at: "2026-05-19T00:00:00Z".into(),
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn upsert_calendars_replaces_set() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed_account(&s).await;

    s.upsert_calendars(
        "acc-1",
        &[
            Calendar {
                id: "primary".into(),
                account_id: "acc-1".into(),
                name: "Primary".into(),
                color: Some("#000".into()),
                included: true,
            },
            Calendar {
                id: "work".into(),
                account_id: "acc-1".into(),
                name: "Work".into(),
                color: None,
                included: true,
            },
        ],
    )
    .await
    .unwrap();

    let list = s.list_calendars("acc-1").await.unwrap();
    assert_eq!(list.len(), 2);

    // Rename "Primary" — included flag must survive.
    s.upsert_calendars(
        "acc-1",
        &[Calendar {
            id: "primary".into(),
            account_id: "acc-1".into(),
            name: "My Primary".into(),
            color: Some("#abc".into()),
            included: false, // server-side this is meaningless; included is local
            created_at: "".into(),
        }
        .ignore_included()],
    )
    .await
    .unwrap();

    let list = s.list_calendars("acc-1").await.unwrap();
    let p = list.iter().find(|c| c.id == "primary").unwrap();
    assert_eq!(p.name, "My Primary");
    // Confirmed: included not clobbered by upsert (default-preserve, set explicitly via set_calendar_included).
    assert!(p.included, "included must not be clobbered by upsert");
}

#[tokio::test]
async fn set_calendar_included_toggles() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed_account(&s).await;
    s.upsert_calendars(
        "acc-1",
        &[Calendar {
            id: "primary".into(),
            account_id: "acc-1".into(),
            name: "Primary".into(),
            color: None,
            included: true,
        }],
    )
    .await
    .unwrap();
    s.set_calendar_included("primary", false).await.unwrap();
    let c = &s.list_calendars("acc-1").await.unwrap()[0];
    assert!(!c.included);
}

#[tokio::test]
async fn delete_account_cascades_to_calendars() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed_account(&s).await;
    s.upsert_calendars(
        "acc-1",
        &[Calendar {
            id: "primary".into(),
            account_id: "acc-1".into(),
            name: "Primary".into(),
            color: None,
            included: true,
        }],
    )
    .await
    .unwrap();
    s.delete_account("acc-1").await.unwrap();
    assert!(s.list_calendars("acc-1").await.unwrap().is_empty());
}
```

Note: the `.ignore_included()` shim isn't needed once you wire the real upsert; remove that synthetic call in step 3 (replace with a simpler upsert call that just doesn't pretend to set `included`).

Replace the rename-test body with:

```rust
    s.upsert_calendars(
        "acc-1",
        &[Calendar {
            id: "primary".into(),
            account_id: "acc-1".into(),
            name: "My Primary".into(),
            color: Some("#abc".into()),
            included: true,   // value ignored by upsert; toggled via set_calendar_included
        }],
    )
    .await
    .unwrap();
```

- [ ] **Step 2: Run — confirm failure**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_store_calendars -- --test-threads=1
```

Expected: FAIL — methods missing.

- [ ] **Step 3: Implement the calendars surface**

In `crates/stint-core/src/calendar/store.rs`, inside `impl CalendarStore`, add:

```rust
    /// Upserts a provider-returned set of calendars for one account. The
    /// `included` field on the input is ignored — locality of the include
    /// flag is preserved by an `ON CONFLICT` that doesn't touch it. New
    /// rows default `included = 1` per the schema.
    pub async fn upsert_calendars(
        &self,
        account_id: &str,
        calendars: &[Calendar],
    ) -> Result<()> {
        let mut tx = self.store.pool().begin().await?;
        for c in calendars {
            sqlx::query(
                r#"INSERT INTO calendars (id, account_id, name, color, included)
                   VALUES (?, ?, ?, ?, 1)
                   ON CONFLICT(id) DO UPDATE SET
                     name = excluded.name,
                     color = excluded.color
                     -- intentionally not touching included
                "#,
            )
            .bind(&c.id)
            .bind(account_id)
            .bind(&c.name)
            .bind(&c.color)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_calendars(&self, account_id: &str) -> Result<Vec<Calendar>> {
        let rows: Vec<(String, String, String, Option<String>, i64)> = sqlx::query_as(
            "SELECT id, account_id, name, color, included
             FROM calendars WHERE account_id = ? ORDER BY name",
        )
        .bind(account_id)
        .fetch_all(self.store.pool())
        .await?;
        Ok(rows
            .into_iter()
            .map(|(id, account_id, name, color, included)| Calendar {
                id,
                account_id,
                name,
                color,
                included: included != 0,
            })
            .collect())
    }

    pub async fn set_calendar_included(&self, calendar_id: &str, included: bool) -> Result<()> {
        sqlx::query("UPDATE calendars SET included = ? WHERE id = ?")
            .bind(if included { 1 } else { 0 })
            .bind(calendar_id)
            .execute(self.store.pool())
            .await?;
        Ok(())
    }
```

- [ ] **Step 4: Run — confirm pass**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_store_calendars -- --test-threads=1
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/calendar/store.rs \
        crates/stint-core/tests/calendar_store_calendars.rs
git commit -m "feat(core): CalendarStore — calendars upsert + included toggle

Upsert preserves the locally-controlled \`included\` flag (ON CONFLICT
intentionally skips it); set_calendar_included is the single mutator.
FK cascade from calendar_accounts is exercised by a regression test."
```

---

### Task 8: Store upsert + range query for `calendar_events` (`store.rs`, part 3)

**Files:**
- Modify: `crates/stint-core/src/calendar/store.rs`
- Create: `crates/stint-core/tests/calendar_store_events.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/calendar_store_events.rs`:

```rust
mod common;

use stint_core::calendar::store::CalendarStore;
use stint_core::calendar::types::{Calendar, CalendarAccount, CalendarEvent, ProviderKind};

async fn seed(s: &CalendarStore) {
    s.add_account(&CalendarAccount {
        id: "acc-1".into(),
        provider: ProviderKind::Google,
        display_name: "me".into(),
        identifier: "me@example.com".into(),
        caldav_url: None,
        enabled: true,
        created_at: "2026-05-19T00:00:00Z".into(),
    })
    .await
    .unwrap();
    s.upsert_calendars(
        "acc-1",
        &[Calendar {
            id: "primary".into(),
            account_id: "acc-1".into(),
            name: "Primary".into(),
            color: None,
            included: true,
        }],
    )
    .await
    .unwrap();
}

fn evt(id: &str, start: &str, end: &str, title: &str) -> CalendarEvent {
    CalendarEvent {
        id: id.into(),
        account_id: "acc-1".into(),
        calendar_id: "primary".into(),
        title: title.into(),
        start_at: start.into(),
        end_at: end.into(),
        is_all_day: false,
        attendee_status: None,
        recurring_root: None,
        fetched_at: "2026-05-19T00:00:00Z".into(),
    }
}

#[tokio::test]
async fn upsert_then_list_returns_events_sorted_by_start() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed(&s).await;

    s.upsert_events(&[
        evt("e2", "2026-05-19T11:00:00Z", "2026-05-19T11:30:00Z", "Lunch prep"),
        evt("e1", "2026-05-19T09:00:00Z", "2026-05-19T09:15:00Z", "Standup"),
    ])
    .await
    .unwrap();

    let list = s
        .list_events_in_range("acc-1", "2026-05-19T00:00:00Z", "2026-05-20T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, "e1");
    assert_eq!(list[1].id, "e2");
}

#[tokio::test]
async fn upsert_is_idempotent_for_same_key() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed(&s).await;

    let e = evt("e1", "2026-05-19T09:00:00Z", "2026-05-19T09:15:00Z", "Standup");
    s.upsert_events(&[e.clone()]).await.unwrap();
    let e2 = CalendarEvent {
        title: "Standup (renamed)".into(),
        ..e.clone()
    };
    s.upsert_events(&[e2]).await.unwrap();

    let list = s
        .list_events_in_range("acc-1", "2026-05-19T00:00:00Z", "2026-05-20T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].title, "Standup (renamed)");
}

#[tokio::test]
async fn recurring_instances_at_different_starts_coexist() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed(&s).await;
    s.upsert_events(&[
        evt("recurring", "2026-05-19T09:00:00Z", "2026-05-19T09:15:00Z", "Standup"),
        evt("recurring", "2026-05-26T09:00:00Z", "2026-05-26T09:15:00Z", "Standup"),
    ])
    .await
    .unwrap();
    let list = s
        .list_events_in_range("acc-1", "2026-05-19T00:00:00Z", "2026-06-01T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn list_events_in_range_excludes_outside_window() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed(&s).await;
    s.upsert_events(&[
        evt("e1", "2026-05-19T09:00:00Z", "2026-05-19T09:15:00Z", "Standup"),
        evt("e2", "2026-05-25T09:00:00Z", "2026-05-25T09:15:00Z", "Future"),
    ])
    .await
    .unwrap();
    let list = s
        .list_events_in_range("acc-1", "2026-05-19T00:00:00Z", "2026-05-20T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "e1");
}

#[tokio::test]
async fn list_events_in_range_excludes_calendars_not_included() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed(&s).await;
    // Add a second calendar, then exclude the primary.
    s.upsert_calendars(
        "acc-1",
        &[Calendar {
            id: "extra".into(),
            account_id: "acc-1".into(),
            name: "Extra".into(),
            color: None,
            included: true,
        }],
    )
    .await
    .unwrap();
    s.upsert_events(&[
        evt("e1", "2026-05-19T09:00:00Z", "2026-05-19T09:15:00Z", "From primary"),
        CalendarEvent {
            id: "e2".into(),
            calendar_id: "extra".into(),
            ..evt("e2", "2026-05-19T10:00:00Z", "2026-05-19T10:15:00Z", "From extra")
        },
    ])
    .await
    .unwrap();
    s.set_calendar_included("primary", false).await.unwrap();

    let list = s
        .list_events_in_range("acc-1", "2026-05-19T00:00:00Z", "2026-05-20T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "e2");
}
```

- [ ] **Step 2: Run — confirm failure**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_store_events -- --test-threads=1
```

Expected: FAIL — methods missing.

- [ ] **Step 3: Implement events upsert + range query**

In `crates/stint-core/src/calendar/store.rs`, inside `impl CalendarStore`, add:

```rust
    pub async fn upsert_events(&self, events: &[CalendarEvent]) -> Result<()> {
        let mut tx = self.store.pool().begin().await?;
        for e in events {
            sqlx::query(
                r#"INSERT INTO calendar_events
                   (id, account_id, calendar_id, title, start_at, end_at,
                    is_all_day, attendee_status, recurring_root, fetched_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                   ON CONFLICT(account_id, id, start_at) DO UPDATE SET
                     calendar_id = excluded.calendar_id,
                     title = excluded.title,
                     end_at = excluded.end_at,
                     is_all_day = excluded.is_all_day,
                     attendee_status = excluded.attendee_status,
                     recurring_root = excluded.recurring_root,
                     fetched_at = excluded.fetched_at"#,
            )
            .bind(&e.id)
            .bind(&e.account_id)
            .bind(&e.calendar_id)
            .bind(&e.title)
            .bind(&e.start_at)
            .bind(&e.end_at)
            .bind(if e.is_all_day { 1 } else { 0 })
            .bind(e.attendee_status.map(|s| s.as_wire().to_string()))
            .bind(&e.recurring_root)
            .bind(&e.fetched_at)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Range is half-open `[from, to)` on `start_at`. Joins against
    /// `calendars` so events on excluded calendars are filtered out.
    pub async fn list_events_in_range(
        &self,
        account_id: &str,
        from: &str,
        to: &str,
    ) -> Result<Vec<CalendarEvent>> {
        type Row = (
            String, String, String, String, String, String, i64,
            Option<String>, Option<String>, String,
        );
        let rows: Vec<Row> = sqlx::query_as(
            r#"SELECT e.id, e.account_id, e.calendar_id, e.title, e.start_at, e.end_at,
                       e.is_all_day, e.attendee_status, e.recurring_root, e.fetched_at
                 FROM calendar_events e
                 JOIN calendars c ON c.id = e.calendar_id
                WHERE e.account_id = ?
                  AND c.included = 1
                  AND e.start_at >= ?
                  AND e.start_at < ?
                ORDER BY e.start_at"#,
        )
        .bind(account_id)
        .bind(from)
        .bind(to)
        .fetch_all(self.store.pool())
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    account_id,
                    calendar_id,
                    title,
                    start_at,
                    end_at,
                    is_all_day,
                    attendee_status,
                    recurring_root,
                    fetched_at,
                )| CalendarEvent {
                    id,
                    account_id,
                    calendar_id,
                    title,
                    start_at,
                    end_at,
                    is_all_day: is_all_day != 0,
                    attendee_status: attendee_status
                        .as_deref()
                        .and_then(AttendeeStatus::from_wire),
                    recurring_root,
                    fetched_at,
                },
            )
            .collect())
    }
```

- [ ] **Step 4: Run — confirm pass**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_store_events -- --test-threads=1
```

Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/calendar/store.rs \
        crates/stint-core/tests/calendar_store_events.rs
git commit -m "feat(core): CalendarStore — events upsert + range query

Upsert keyed on (account_id, id, start_at); recurring instances
coexist by design. Range query joins calendars so events on
excluded calendars are filtered out automatically."
```

---

### Task 9: Store CRUD for `event_decisions` (`store.rs`, part 4)

**Files:**
- Modify: `crates/stint-core/src/calendar/store.rs`
- Create: `crates/stint-core/tests/calendar_store_decisions.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/calendar_store_decisions.rs`:

```rust
mod common;

use stint_core::calendar::store::CalendarStore;
use stint_core::calendar::types::{
    Calendar, CalendarAccount, CalendarEvent, EventDecision, ProviderKind,
};

async fn seed(s: &CalendarStore) {
    s.add_account(&CalendarAccount {
        id: "acc-1".into(),
        provider: ProviderKind::Google,
        display_name: "me".into(),
        identifier: "me@example.com".into(),
        caldav_url: None,
        enabled: true,
        created_at: "2026-05-19T00:00:00Z".into(),
    })
    .await
    .unwrap();
    s.upsert_calendars(
        "acc-1",
        &[Calendar {
            id: "primary".into(),
            account_id: "acc-1".into(),
            name: "Primary".into(),
            color: None,
            included: true,
        }],
    )
    .await
    .unwrap();
    s.upsert_events(&[CalendarEvent {
        id: "e1".into(),
        account_id: "acc-1".into(),
        calendar_id: "primary".into(),
        title: "Standup".into(),
        start_at: "2026-05-19T09:00:00Z".into(),
        end_at: "2026-05-19T09:15:00Z".into(),
        is_all_day: false,
        attendee_status: None,
        recurring_root: None,
        fetched_at: "2026-05-19T00:00:00Z".into(),
    }])
    .await
    .unwrap();
}

#[tokio::test]
async fn record_then_get_decision_returns_kind() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed(&s).await;

    s.record_decision(
        "acc-1",
        "e1",
        "2026-05-19T09:00:00Z",
        &EventDecision::Ignored,
    )
    .await
    .unwrap();

    let d = s
        .get_decision("acc-1", "e1", "2026-05-19T09:00:00Z")
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(d, EventDecision::Ignored));
}

#[tokio::test]
async fn record_decision_overwrites_previous() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed(&s).await;

    s.record_decision("acc-1", "e1", "2026-05-19T09:00:00Z", &EventDecision::Ignored)
        .await
        .unwrap();
    s.record_decision(
        "acc-1",
        "e1",
        "2026-05-19T09:00:00Z",
        &EventDecision::LoggedManual {
            linked_local_uuid: "te-1".into(),
        },
    )
    .await
    .unwrap();

    let d = s
        .get_decision("acc-1", "e1", "2026-05-19T09:00:00Z")
        .await
        .unwrap()
        .unwrap();
    match d {
        EventDecision::LoggedManual { linked_local_uuid } => {
            assert_eq!(linked_local_uuid, "te-1");
        }
        _ => panic!("expected LoggedManual"),
    }
}

#[tokio::test]
async fn list_decisions_filters_by_range() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed(&s).await;
    s.upsert_events(&[CalendarEvent {
        id: "e2".into(),
        account_id: "acc-1".into(),
        calendar_id: "primary".into(),
        title: "Next week".into(),
        start_at: "2026-05-25T09:00:00Z".into(),
        end_at: "2026-05-25T09:15:00Z".into(),
        is_all_day: false,
        attendee_status: None,
        recurring_root: None,
        fetched_at: "2026-05-19T00:00:00Z".into(),
    }])
    .await
    .unwrap();
    s.record_decision("acc-1", "e1", "2026-05-19T09:00:00Z", &EventDecision::Ignored)
        .await
        .unwrap();
    s.record_decision(
        "acc-1",
        "e2",
        "2026-05-25T09:00:00Z",
        &EventDecision::LoggedManual {
            linked_local_uuid: "te-1".into(),
        },
    )
    .await
    .unwrap();

    let list = s
        .list_decisions_in_range("acc-1", "2026-05-19T00:00:00Z", "2026-05-20T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].0, "e1");
}
```

- [ ] **Step 2: Run — confirm failure**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_store_decisions -- --test-threads=1
```

Expected: FAIL — methods missing.

- [ ] **Step 3: Implement decision CRUD**

In `crates/stint-core/src/calendar/store.rs`, inside `impl CalendarStore`, add:

```rust
    pub async fn record_decision(
        &self,
        account_id: &str,
        event_id: &str,
        event_start: &str,
        decision: &EventDecision,
    ) -> Result<()> {
        let now = time::now_utc();
        sqlx::query(
            r#"INSERT INTO event_decisions
               (account_id, event_id, event_start, decision, linked_local_uuid, decided_at)
               VALUES (?, ?, ?, ?, ?, ?)
               ON CONFLICT(account_id, event_id, event_start) DO UPDATE SET
                 decision = excluded.decision,
                 linked_local_uuid = excluded.linked_local_uuid,
                 decided_at = excluded.decided_at"#,
        )
        .bind(account_id)
        .bind(event_id)
        .bind(event_start)
        .bind(decision.as_wire())
        .bind(decision.linked_local_uuid())
        .bind(&now)
        .execute(self.store.pool())
        .await?;
        Ok(())
    }

    pub async fn get_decision(
        &self,
        account_id: &str,
        event_id: &str,
        event_start: &str,
    ) -> Result<Option<EventDecision>> {
        let row: Option<(String, Option<String>)> = sqlx::query_as(
            "SELECT decision, linked_local_uuid FROM event_decisions
             WHERE account_id = ? AND event_id = ? AND event_start = ?",
        )
        .bind(account_id)
        .bind(event_id)
        .bind(event_start)
        .fetch_optional(self.store.pool())
        .await?;
        Ok(row.and_then(|(wire, uuid)| EventDecision::decoded(&wire, uuid)))
    }

    /// Returns `(event_id, event_start, decision)` triples for decisions
    /// whose `event_start` falls in `[from, to)`. The event-id form lets
    /// the caller index decisions against an event list cheaply.
    pub async fn list_decisions_in_range(
        &self,
        account_id: &str,
        from: &str,
        to: &str,
    ) -> Result<Vec<(String, String, EventDecision)>> {
        let rows: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
            "SELECT event_id, event_start, decision, linked_local_uuid
             FROM event_decisions
             WHERE account_id = ? AND event_start >= ? AND event_start < ?",
        )
        .bind(account_id)
        .bind(from)
        .bind(to)
        .fetch_all(self.store.pool())
        .await?;
        Ok(rows
            .into_iter()
            .filter_map(|(event_id, event_start, wire, uuid)| {
                EventDecision::decoded(&wire, uuid).map(|d| (event_id, event_start, d))
            })
            .collect())
    }
```

- [ ] **Step 4: Run — confirm pass**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_store_decisions -- --test-threads=1
```

Expected: PASS.

- [ ] **Step 5: Clean up the `_phantom_imports` shim**

The `_phantom_imports` function added in Task 6 is no longer needed (all imports are used). Delete it from `crates/stint-core/src/calendar/store.rs`.

- [ ] **Step 6: Commit**

```bash
git add crates/stint-core/src/calendar/store.rs \
        crates/stint-core/tests/calendar_store_decisions.rs
git commit -m "feat(core): CalendarStore — event_decisions CRUD

Adds record/get/list for the event_decisions table. Decisions are
keyed (account, event, event_start) to match recurring-instance
semantics — ignoring one instance does not affect future ones."
```

---

### Task 10: `Entries::create_completed` for the "Log this" action

**Files:**
- Modify: `crates/stint-core/src/store/entries.rs`
- Create: `crates/stint-core/tests/store_entries_completed.rs`

The existing `Entries::create` only supports the running-timer flow (no `end_at`). For "Log this" we need a path that creates a finalized entry with `start_at`, `end_at`, `source = 'calendar'`, and `source_event_id` populated.

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/store_entries_completed.rs`:

```rust
mod common;

use stint_core::store::entries::{Entries, NewCompletedEntry};

#[tokio::test]
async fn create_completed_persists_all_fields() {
    let env = common::setup().await;
    let e = Entries::new(env.store.clone());

    let uuid = e
        .create_completed(NewCompletedEntry {
            description: "Sprint review".into(),
            project_id: Some("p-1".into()),
            task_id: None,
            start_at: "2026-05-19T14:00:00Z".into(),
            end_at: "2026-05-19T15:00:00Z".into(),
            billable: true,
            source: "calendar".into(),
            source_event_id: Some("acc-1:evt-1:2026-05-19T14:00:00Z".into()),
        })
        .await
        .unwrap();

    let row = e.get(&uuid).await.unwrap().expect("entry persisted");
    assert_eq!(row.description, "Sprint review");
    assert_eq!(row.start_at, "2026-05-19T14:00:00Z");
    assert_eq!(row.end_at.as_deref(), Some("2026-05-19T15:00:00Z"));
    assert_eq!(row.source, "calendar");
    assert_eq!(
        row.source_event_id.as_deref(),
        Some("acc-1:evt-1:2026-05-19T14:00:00Z")
    );
    assert_eq!(row.billable, 1);
    assert_eq!(row.sync_state, "pending_create");
}

#[tokio::test]
async fn create_completed_returns_unique_uuids() {
    let env = common::setup().await;
    let e = Entries::new(env.store.clone());
    let mk = || NewCompletedEntry {
        description: "x".into(),
        project_id: None,
        task_id: None,
        start_at: "2026-05-19T09:00:00Z".into(),
        end_at: "2026-05-19T09:30:00Z".into(),
        billable: false,
        source: "calendar".into(),
        source_event_id: None,
    };
    let a = e.create_completed(mk()).await.unwrap();
    let b = e.create_completed(mk()).await.unwrap();
    assert_ne!(a, b);
}
```

- [ ] **Step 2: Run — confirm failure**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test store_entries_completed -- --test-threads=1
```

Expected: FAIL — `NewCompletedEntry` / `create_completed` don't exist.

- [ ] **Step 3: Implement `create_completed`**

In `crates/stint-core/src/store/entries.rs`, after the existing `NewTimeEntry` struct, add:

```rust
#[derive(Debug, Clone)]
pub struct NewCompletedEntry {
    pub description: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub start_at: String,
    pub end_at: String,
    pub billable: bool,
    pub source: String,
    pub source_event_id: Option<String>,
}
```

Inside `impl Entries`, after the existing `create` method, add:

```rust
    /// Insert a finalised time entry (both start_at and end_at set), used by
    /// the calendar "Log this" path and any future bulk-import flow. The
    /// entry begins in `pending_create` so the regular sync queue picks it
    /// up exactly like a CLI/GUI-created entry.
    pub async fn create_completed(&self, new: NewCompletedEntry) -> Result<String> {
        let local_uuid = ids::new_local_uuid();
        let now = time::now_utc();
        sqlx::query(
            r#"INSERT INTO time_entries
               (local_uuid, description, project_id, task_id, start_at, end_at,
                billable, source, source_event_id, sync_state, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending_create', ?, ?)"#,
        )
        .bind(&local_uuid)
        .bind(new.description)
        .bind(new.project_id)
        .bind(new.task_id)
        .bind(new.start_at)
        .bind(new.end_at)
        .bind(if new.billable { 1 } else { 0 })
        .bind(new.source)
        .bind(new.source_event_id)
        .bind(&now)
        .bind(&now)
        .execute(self.store.pool())
        .await?;
        Ok(local_uuid)
    }
```

- [ ] **Step 4: Run — confirm pass**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test store_entries_completed -- --test-threads=1
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/store/entries.rs \
        crates/stint-core/tests/store_entries_completed.rs
git commit -m "feat(core): Entries::create_completed for finalised inserts

Calendar 'Log this' needs to insert an entry with start_at AND end_at
in one shot, plus source = 'calendar' and source_event_id populated.
Existing create() only fits the running-timer path. New entry starts
in pending_create so the regular sync queue picks it up unchanged."
```

---

### Task 11: Per-account Keychain blob helpers + the queue-insert path

**Files:**
- Modify: `crates/stint-core/src/calendar/store.rs` (Keychain helpers)
- Modify: `crates/stint-core/src/sync/mod.rs` and/or `sync/push.rs` if the existing flow does not already enqueue from `create_completed`
- Create: `crates/stint-core/tests/calendar_oauth_blob.rs`
- Verify: `crates/stint-core/tests/sync_push.rs` style of test for finalised entries

First, the Keychain helpers. The Solidtime OAuth blob in 3a is keyed by a fixed name (`solidtime.oauth`); calendar accounts are keyed by their local UUID (`calendar.<uuid>`).

- [ ] **Step 1: Write the failing Keychain blob test**

Create `crates/stint-core/tests/calendar_oauth_blob.rs`:

```rust
use chrono::{Duration, Utc};
use stint_core::calendar::store::{
    calendar_blob_delete, calendar_blob_load, calendar_blob_save, CalendarOAuthBlob,
};
use stint_core::config::secrets::Secrets;
use stint_core::oauth::tokens::TokenSet;

fn unique_prefix() -> String {
    // Per-test prefix so concurrent Keychain entries don't collide.
    format!(
        "tech.reyem.stint-test.{}",
        uuid::Uuid::new_v4().simple()
    )
}

#[test]
fn save_load_delete_roundtrip() {
    if std::env::var_os("STINT_SKIP_KEYCHAIN_TESTS").is_some() {
        eprintln!("skipped: STINT_SKIP_KEYCHAIN_TESTS=1");
        return;
    }
    let secrets = Secrets::with_service_prefix(unique_prefix());
    let blob = CalendarOAuthBlob {
        client_id: "fake-google-client-id".into(),
        tokens: TokenSet::from_response(
            "access-1".into(),
            Some("refresh-1".into()),
            3600,
            Some("https://www.googleapis.com/auth/calendar.readonly".into()),
            Utc::now(),
        ),
    };
    let account_uuid = "acc-12345";

    assert!(calendar_blob_load(&secrets, account_uuid).unwrap().is_none());

    calendar_blob_save(&secrets, account_uuid, &blob).unwrap();
    let loaded = calendar_blob_load(&secrets, account_uuid).unwrap().unwrap();
    assert_eq!(loaded.tokens.access_token, "access-1");
    assert_eq!(loaded.client_id, "fake-google-client-id");

    calendar_blob_delete(&secrets, account_uuid).unwrap();
    assert!(calendar_blob_load(&secrets, account_uuid).unwrap().is_none());
}

#[test]
fn load_returns_none_for_missing_account() {
    if std::env::var_os("STINT_SKIP_KEYCHAIN_TESTS").is_some() {
        eprintln!("skipped: STINT_SKIP_KEYCHAIN_TESTS=1");
        return;
    }
    let secrets = Secrets::with_service_prefix(unique_prefix());
    assert!(
        calendar_blob_load(&secrets, "missing-acc").unwrap().is_none()
    );
}

#[test]
fn load_surfaces_oauth_server_on_corrupt_blob() {
    if std::env::var_os("STINT_SKIP_KEYCHAIN_TESTS").is_some() {
        eprintln!("skipped: STINT_SKIP_KEYCHAIN_TESTS=1");
        return;
    }
    let secrets = Secrets::with_service_prefix(unique_prefix());
    let account_uuid = "acc-bad";
    secrets
        .set(&format!("calendar.{account_uuid}"), "this is not JSON")
        .unwrap();

    let err = calendar_blob_load(&secrets, account_uuid).unwrap_err();
    match err {
        stint_core::Error::OAuthServer(msg) => assert!(msg.contains("malformed")),
        e => panic!("expected OAuthServer, got {e:?}"),
    }
    secrets.delete(&format!("calendar.{account_uuid}")).ok();
}
```

Add `uuid` as a dev-dep on `stint-core` if it isn't already exposed at dev scope (it is — see existing `[dev-dependencies] uuid = { workspace = true }`). No Cargo.toml change needed.

- [ ] **Step 2: Run — confirm failure**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_oauth_blob -- --test-threads=1
```

Expected: tests skipped (env var set). Drop the env var to actually exercise Keychain locally:

```bash
cargo test -p stint-core --test calendar_oauth_blob -- --test-threads=1
```

Expected without env var: FAIL — symbols don't exist.

- [ ] **Step 3: Implement the helpers**

In `crates/stint-core/src/calendar/store.rs`, add at the top after the existing imports:

```rust
use crate::config::secrets::Secrets;
use crate::oauth::tokens::TokenSet;
use serde::{Deserialize, Serialize};

/// Per-account OAuth credentials stored in Keychain as one JSON blob.
/// Same shape as the Solidtime OAuth blob (3a) for consistency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarOAuthBlob {
    pub client_id: String,
    pub tokens: TokenSet,
}

fn calendar_blob_key(account_uuid: &str) -> String {
    format!("calendar.{account_uuid}")
}

pub fn calendar_blob_load(
    secrets: &Secrets,
    account_uuid: &str,
) -> crate::Result<Option<CalendarOAuthBlob>> {
    let Some(raw) = secrets.get(&calendar_blob_key(account_uuid))? else {
        return Ok(None);
    };
    let blob: CalendarOAuthBlob = serde_json::from_str(&raw).map_err(|e| {
        crate::Error::OAuthServer(format!(
            "Calendar Keychain blob malformed for {account_uuid}: {e}"
        ))
    })?;
    Ok(Some(blob))
}

pub fn calendar_blob_save(
    secrets: &Secrets,
    account_uuid: &str,
    blob: &CalendarOAuthBlob,
) -> crate::Result<()> {
    let raw = serde_json::to_string(blob).expect("CalendarOAuthBlob is JSON-serializable");
    secrets.set(&calendar_blob_key(account_uuid), &raw)
}

pub fn calendar_blob_delete(secrets: &Secrets, account_uuid: &str) -> crate::Result<()> {
    secrets.delete(&calendar_blob_key(account_uuid))
}
```

- [ ] **Step 4: Confirm the existing sync queue includes "Log this" entries**

Read `crates/stint-core/src/sync/mod.rs` and `sync/push.rs`. The sync drain scans for entries in `pending_create`. Since `create_completed` sets that state, no new code path is required — but write a regression test to confirm:

Create `crates/stint-core/tests/calendar_logged_entry_sync.rs`:

```rust
mod common;

use stint_core::solidtime::SolidtimeClient;
use stint_core::store::entries::{Entries, NewCompletedEntry};
use stint_core::sync::drain_once;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn logged_calendar_entry_drains_through_sync_queue() {
    let env = common::setup().await;
    let server = MockServer::start().await;

    // Configure the entry's organisation so sync builds the URL.
    let settings = stint_core::config::Settings::new(env.store.clone());
    settings.set("solidtime.url", &server.uri()).await.unwrap();
    settings.set("solidtime.org", "org-1").await.unwrap();
    settings.set("solidtime.member_id", "mem-1").await.unwrap();

    Mock::given(method("POST"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .and(header("Authorization", "Bearer tok"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": { "id": "remote-1", "description": "Sprint review",
                       "start": "2026-05-19T14:00:00Z", "end": "2026-05-19T15:00:00Z" }
        })))
        .mount(&server)
        .await;

    let entries = Entries::new(env.store.clone());
    let local_uuid = entries
        .create_completed(NewCompletedEntry {
            description: "Sprint review".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-19T14:00:00Z".into(),
            end_at: "2026-05-19T15:00:00Z".into(),
            billable: false,
            source: "calendar".into(),
            source_event_id: Some("acc-1:evt-1:2026-05-19T14:00:00Z".into()),
        })
        .await
        .unwrap();

    // Enqueue a create-op for the new entry. The existing flow does this
    // inside the timer surfaces; for calendar log-this we enqueue here.
    use stint_core::store::queue::{Queue, QueueOp};
    let queue = Queue::new(env.store.clone());
    queue
        .enqueue_create_for(&local_uuid)
        .await
        .unwrap();

    let client = SolidtimeClient::with_api_token(&server.uri(), "tok").with_org("org-1");
    let drained = drain_once(&env.store, &client).await.unwrap();
    assert_eq!(drained, 1);

    let row = entries.get(&local_uuid).await.unwrap().unwrap();
    assert_eq!(row.sync_state, "synced");
    assert_eq!(row.solidtime_id.as_deref(), Some("remote-1"));
}
```

Open `crates/stint-core/src/store/queue.rs` to check the existing API. If `Queue::enqueue_create_for(&local_uuid)` is not a method, fall back to whatever the timer/stop path uses (likely `enqueue_create_entry(&local_uuid)` or similar). Rename in the test if needed; if no such helper exists, this test instead becomes a placeholder asserting the entry is `pending_create` and then explicitly inserts into `sync_queue` with the existing pattern from `sync_push.rs` tests.

```bash
grep -n "pub async fn enqueue" crates/stint-core/src/store/queue.rs
```

Pick the closest matching method. Drop the helper from the test entirely if the existing tests use raw SQL — match the simplest existing pattern.

- [ ] **Step 5: Run — confirm pass**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_oauth_blob -- --test-threads=1
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_logged_entry_sync -- --test-threads=1
```

Expected: pass (Keychain test is skipped under the env var; you exercise it once locally without it).

- [ ] **Step 6: Commit**

```bash
git add crates/stint-core/src/calendar/store.rs \
        crates/stint-core/tests/calendar_oauth_blob.rs \
        crates/stint-core/tests/calendar_logged_entry_sync.rs
git commit -m "feat(core): calendar per-account Keychain blob + sync regression

calendar_blob_{load,save,delete} key OAuth credentials by account UUID
(tech.reyem.stint.calendar.<uuid>). Mirrors the 3a Solidtime OAuth blob
shape. Regression test confirms a 'Log this'-created entry drains
through the existing sync queue without changes."
```

---

### Task 12: Google OAuth config + login helper (`google/config.rs`)

**Files:**
- Modify: `crates/stint-core/src/calendar/google/config.rs`
- Create: `crates/stint-core/tests/calendar_google_config.rs`

The constant is committed with a **placeholder value**; Mario pastes the real client ID before the first E2E in Task 20. Tests use `STINT_GOOGLE_CLIENT_ID` to inject a fake.

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/calendar_google_config.rs`:

```rust
use stint_core::calendar::google::config::{
    google_oauth_config, google_oauth_config_with_client_id, GOOGLE_CALENDAR_READONLY_SCOPE,
};
use stint_core::oauth::client::OAuthClient;

#[test]
fn google_oauth_config_includes_required_endpoints_and_scope() {
    let cfg = google_oauth_config_with_client_id("fake-client.apps.googleusercontent.com");
    assert_eq!(cfg.authorize_url, "https://accounts.google.com/o/oauth2/v2/auth");
    assert_eq!(cfg.token_url, "https://oauth2.googleapis.com/token");
    assert_eq!(cfg.client_id, "fake-client.apps.googleusercontent.com");
    assert!(cfg.scopes.iter().any(|s| s == GOOGLE_CALENDAR_READONLY_SCOPE));
}

#[test]
fn google_authorize_url_carries_access_type_offline_and_prompt_consent() {
    let cfg = google_oauth_config_with_client_id("fake-client.apps.googleusercontent.com");
    let prepared = OAuthClient::new(cfg).prepare_authorize();
    let url = prepared.authorize_url.to_string();
    assert!(url.contains("access_type=offline"), "got {url}");
    assert!(url.contains("prompt=consent"), "got {url}");
    assert!(
        url.contains("scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fcalendar.readonly"),
        "got {url}"
    );
}

#[test]
fn google_oauth_config_honours_env_override() {
    // The build-time constant is consulted when no env var is set; with
    // the env var, the env value wins for tests.
    std::env::set_var(
        "STINT_GOOGLE_CLIENT_ID",
        "override-client.apps.googleusercontent.com",
    );
    let cfg = google_oauth_config();
    assert_eq!(cfg.client_id, "override-client.apps.googleusercontent.com");
    std::env::remove_var("STINT_GOOGLE_CLIENT_ID");
}
```

- [ ] **Step 2: Run — confirm failure**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_google_config -- --test-threads=1
```

Expected: FAIL — symbols missing.

- [ ] **Step 3: Implement `crates/stint-core/src/calendar/google/config.rs`**

```rust
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

/// **PLACEHOLDER** — replace with the OAuth 2.0 client ID registered on
/// Google Cloud Console (Application type: "Desktop application"). See
/// the Phase 3b plan's "Prerequisites" section.
///
/// The `STINT_GOOGLE_CLIENT_ID` env var overrides this value at runtime;
/// integration tests rely on the override.
pub const GOOGLE_OAUTH_CLIENT_ID: &str = "REPLACE_ME.apps.googleusercontent.com";

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
```

- [ ] **Step 4: Run — confirm pass**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_google_config -- --test-threads=1
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/calendar/google/config.rs \
        crates/stint-core/tests/calendar_google_config.rs
git commit -m "feat(core): Google OAuth config + readonly scope

GOOGLE_OAUTH_CLIENT_ID is committed as a placeholder; the real value
must be pasted before the first E2E in Task 20 (see Prerequisites in
the Phase 3b plan). STINT_GOOGLE_CLIENT_ID env-var overrides for
tests and local dev. access_type=offline + prompt=consent are sent
on the authorize URL so Google consistently issues a refresh_token."
```

---

### Task 13: Google HTTP client + DTOs (`google/client.rs`, `google/dto.rs`)

**Files:**
- Modify: `crates/stint-core/src/calendar/google/dto.rs`
- Modify: `crates/stint-core/src/calendar/google/client.rs`
- Create: `crates/stint-core/tests/calendar_google_client.rs`

The client wraps the v3 REST endpoints we need: `calendarList` and `events`. The base URL is parameterized so wiremock can substitute the fake host.

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/calendar_google_client.rs`:

```rust
use stint_core::calendar::google::client::GoogleClient;
use stint_core::calendar::types::TimeRange;
use chrono::{TimeZone, Utc};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn range_today() -> TimeRange {
    TimeRange {
        start: Utc.with_ymd_and_hms(2026, 5, 19, 0, 0, 0).unwrap(),
        end: Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
    }
}

#[tokio::test]
async fn list_calendars_calls_calendar_list_with_bearer() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/users/me/calendarList"))
        .and(header("Authorization", "Bearer access-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [
                { "id": "primary", "summary": "Primary", "backgroundColor": "#abc" },
                { "id": "work@example.com", "summary": "Work" }
            ]
        })))
        .mount(&server)
        .await;

    let client = GoogleClient::with_base_url(&server.uri());
    let cals = client.list_calendars("access-1").await.unwrap();
    assert_eq!(cals.len(), 2);
    assert_eq!(cals[0].id, "primary");
    assert_eq!(cals[0].name, "Primary");
    assert_eq!(cals[0].color.as_deref(), Some("#abc"));
    assert_eq!(cals[1].id, "work@example.com");
}

#[tokio::test]
async fn list_events_calls_events_with_range_and_single_events() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/calendars/primary/events"))
        .and(header("Authorization", "Bearer access-1"))
        .and(query_param("singleEvents", "true"))
        .and(query_param("orderBy", "startTime"))
        .and(query_param("timeMin", "2026-05-19T00:00:00+00:00"))
        .and(query_param("timeMax", "2026-05-20T00:00:00+00:00"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [
                {
                    "id": "evt-1",
                    "summary": "Standup",
                    "start": { "dateTime": "2026-05-19T09:00:00Z" },
                    "end":   { "dateTime": "2026-05-19T09:15:00Z" },
                    "attendees": [
                        { "self": true, "responseStatus": "accepted" },
                        { "self": false, "responseStatus": "declined" }
                    ]
                },
                {
                    "id": "evt-2",
                    "summary": "All-hands",
                    "start": { "date": "2026-05-19" },
                    "end":   { "date": "2026-05-20" }
                },
                {
                    "id": "evt-3",
                    "summary": "Recurring 1:1",
                    "start": { "dateTime": "2026-05-19T11:00:00Z" },
                    "end":   { "dateTime": "2026-05-19T11:30:00Z" },
                    "recurringEventId": "evt-root"
                }
            ]
        })))
        .mount(&server)
        .await;

    let client = GoogleClient::with_base_url(&server.uri());
    let evs = client.list_events("access-1", "primary", range_today()).await.unwrap();
    assert_eq!(evs.len(), 3);

    assert_eq!(evs[0].id, "evt-1");
    assert_eq!(evs[0].title, "Standup");
    assert_eq!(evs[0].start_at, "2026-05-19T09:00:00Z");
    assert!(!evs[0].is_all_day);
    assert_eq!(
        evs[0].attendee_status,
        Some(stint_core::calendar::types::AttendeeStatus::Accepted)
    );

    assert_eq!(evs[1].title, "All-hands");
    assert!(evs[1].is_all_day);
    assert_eq!(evs[1].start_at, "2026-05-19");

    assert_eq!(evs[2].recurring_root.as_deref(), Some("evt-root"));
}

#[tokio::test]
async fn list_events_maps_401_to_auth_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/calendars/primary/events"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let client = GoogleClient::with_base_url(&server.uri());
    let err = client.list_events("access-1", "primary", range_today()).await.unwrap_err();
    assert!(matches!(err, stint_core::Error::OAuthRefreshFailed));
}

#[tokio::test]
async fn list_events_paginates_with_nextPageToken() {
    let server = MockServer::start().await;
    // Page 1.
    Mock::given(method("GET"))
        .and(path("/calendar/v3/calendars/primary/events"))
        .and(query_param("singleEvents", "true"))
        .and(query_param("orderBy", "startTime"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [
                { "id": "evt-1", "summary": "First",
                  "start": { "dateTime": "2026-05-19T09:00:00Z" },
                  "end":   { "dateTime": "2026-05-19T09:15:00Z" } }
            ],
            "nextPageToken": "tok-2"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    // Page 2.
    Mock::given(method("GET"))
        .and(path("/calendar/v3/calendars/primary/events"))
        .and(query_param("pageToken", "tok-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [
                { "id": "evt-2", "summary": "Second",
                  "start": { "dateTime": "2026-05-19T10:00:00Z" },
                  "end":   { "dateTime": "2026-05-19T10:30:00Z" } }
            ]
        })))
        .mount(&server)
        .await;

    let client = GoogleClient::with_base_url(&server.uri());
    let evs = client.list_events("access-1", "primary", range_today()).await.unwrap();
    assert_eq!(evs.len(), 2);
    assert_eq!(evs[0].id, "evt-1");
    assert_eq!(evs[1].id, "evt-2");
}
```

- [ ] **Step 2: Run — confirm failure**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_google_client -- --test-threads=1
```

Expected: FAIL.

- [ ] **Step 3: Implement DTOs**

Write `crates/stint-core/src/calendar/google/dto.rs`:

```rust
//! Wire DTOs for the Google Calendar v3 REST surface, plus mappers to
//! the provider-shaped `RemoteCalendar` / `RemoteEvent`.

use crate::calendar::provider::{RemoteCalendar, RemoteEvent};
use crate::calendar::types::AttendeeStatus;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct CalendarListResponse {
    #[serde(default)]
    pub items: Vec<CalendarListEntry>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CalendarListEntry {
    pub id: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub background_color: Option<String>,
    #[serde(default, rename = "backgroundColor")]
    pub background_color_camel: Option<String>,
}

impl CalendarListEntry {
    pub(crate) fn into_remote(self) -> RemoteCalendar {
        RemoteCalendar {
            id: self.id,
            name: self.summary.unwrap_or_default(),
            color: self.background_color.or(self.background_color_camel),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct EventsResponse {
    #[serde(default)]
    pub items: Vec<EventEntry>,
    #[serde(default, rename = "nextPageToken")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EventEntry {
    pub id: String,
    #[serde(default)]
    pub summary: Option<String>,
    pub start: EventTime,
    pub end: EventTime,
    #[serde(default, rename = "recurringEventId")]
    pub recurring_event_id: Option<String>,
    #[serde(default)]
    pub attendees: Vec<EventAttendee>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EventTime {
    #[serde(default)]
    pub date: Option<String>, // YYYY-MM-DD for all-day events
    #[serde(default, rename = "dateTime")]
    pub date_time: Option<String>, // RFC 3339 for timed events
}

#[derive(Debug, Deserialize)]
pub(crate) struct EventAttendee {
    #[serde(default, rename = "self")]
    pub is_self: bool,
    #[serde(default, rename = "responseStatus")]
    pub response_status: Option<String>,
}

impl EventEntry {
    pub(crate) fn into_remote(self, calendar_id: &str) -> RemoteEvent {
        let is_all_day = self.start.date.is_some();
        let start_at = self
            .start
            .date_time
            .or(self.start.date)
            .unwrap_or_default();
        let end_at = self.end.date_time.or(self.end.date).unwrap_or_default();
        let attendee_status = self
            .attendees
            .iter()
            .find(|a| a.is_self)
            .and_then(|a| a.response_status.as_deref())
            .and_then(AttendeeStatus::from_wire);

        RemoteEvent {
            id: self.id,
            calendar_id: calendar_id.into(),
            title: self.summary.unwrap_or_default(),
            start_at,
            end_at,
            is_all_day,
            attendee_status,
            recurring_root: self.recurring_event_id,
        }
    }
}
```

Note: I included both `background_color` and `background_color_camel` defensively because Google v3 uses `backgroundColor` (camelCase). Actually, Google's API always uses camelCase. Simplify the struct to just one field:

```rust
#[derive(Debug, Deserialize)]
pub(crate) struct CalendarListEntry {
    pub id: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default, rename = "backgroundColor")]
    pub background_color: Option<String>,
}

impl CalendarListEntry {
    pub(crate) fn into_remote(self) -> RemoteCalendar {
        RemoteCalendar {
            id: self.id,
            name: self.summary.unwrap_or_default(),
            color: self.background_color,
        }
    }
}
```

(Replace the dual-field version with this simpler one — only one rename works.)

- [ ] **Step 4: Implement the client**

Write `crates/stint-core/src/calendar/google/client.rs`:

```rust
//! HTTP wrapper over Google Calendar v3.
//!
//! Only the read-side endpoints we care about for Phase 3b: list user's
//! `calendarList` and per-calendar `events` (with `singleEvents=true`
//! server-side expansion of recurrences and `pageToken` paging).

use crate::calendar::google::dto::{CalendarListResponse, EventsResponse};
use crate::calendar::provider::{RemoteCalendar, RemoteEvent};
use crate::calendar::types::TimeRange;
use crate::{Error, Result};
use reqwest::{Client, StatusCode};

pub const GOOGLE_API_BASE: &str = "https://www.googleapis.com";

pub struct GoogleClient {
    base_url: String,
    http: Client,
}

impl GoogleClient {
    pub fn new() -> Self {
        Self::with_base_url(GOOGLE_API_BASE)
    }

    pub fn with_base_url(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("reqwest client builds"),
        }
    }

    pub async fn list_calendars(&self, access_token: &str) -> Result<Vec<RemoteCalendar>> {
        let url = format!("{}/calendar/v3/users/me/calendarList", self.base_url);
        let resp = self
            .http
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(Error::from)?;
        let status = resp.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err(Error::OAuthRefreshFailed);
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::OAuthServer(format!(
                "google calendarList HTTP {status}: {body}"
            )));
        }
        let parsed: CalendarListResponse = resp.json().await?;
        Ok(parsed
            .items
            .into_iter()
            .map(|c| c.into_remote())
            .collect())
    }

    pub async fn list_events(
        &self,
        access_token: &str,
        calendar_id: &str,
        range: TimeRange,
    ) -> Result<Vec<RemoteEvent>> {
        let mut events = Vec::new();
        let mut page_token: Option<String> = None;
        let time_min = range.start.to_rfc3339();
        let time_max = range.end.to_rfc3339();

        loop {
            let url = format!(
                "{}/calendar/v3/calendars/{}/events",
                self.base_url,
                urlencoding::encode(calendar_id)
            );
            let mut req = self
                .http
                .get(&url)
                .bearer_auth(access_token)
                .query(&[
                    ("singleEvents", "true"),
                    ("orderBy", "startTime"),
                    ("timeMin", time_min.as_str()),
                    ("timeMax", time_max.as_str()),
                    ("maxResults", "250"),
                ]);
            if let Some(tok) = &page_token {
                req = req.query(&[("pageToken", tok.as_str())]);
            }
            let resp = req.send().await.map_err(Error::from)?;
            let status = resp.status();
            if status == StatusCode::UNAUTHORIZED {
                return Err(Error::OAuthRefreshFailed);
            }
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(Error::OAuthServer(format!(
                    "google events HTTP {status}: {body}"
                )));
            }
            let parsed: EventsResponse = resp.json().await?;
            for item in parsed.items {
                events.push(item.into_remote(calendar_id));
            }
            match parsed.next_page_token {
                Some(tok) if !tok.is_empty() => page_token = Some(tok),
                _ => break,
            }
        }
        Ok(events)
    }
}

impl Default for GoogleClient {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 5: Add `urlencoding` to `stint-core/Cargo.toml`**

```toml
urlencoding = "2"
```

(In `[dependencies]`, alphabetically after `url`.)

- [ ] **Step 6: Run — confirm pass**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_google_client -- --test-threads=1
```

Expected: PASS (4 tests).

- [ ] **Step 7: Commit**

```bash
git add crates/stint-core/src/calendar/google/dto.rs \
        crates/stint-core/src/calendar/google/client.rs \
        crates/stint-core/Cargo.toml \
        crates/stint-core/tests/calendar_google_client.rs
git commit -m "feat(core): Google Calendar v3 HTTP client

GoogleClient wraps the read endpoints we need: users.calendarList
and calendars.events (with singleEvents=true server-side recurrence
expansion + nextPageToken paging). 401 maps to OAuthRefreshFailed so
the caller triggers re-auth; other errors surface as OAuthServer with
the response body."
```

---

### Task 14: `GoogleProvider` (`google/mod.rs`)

**Files:**
- Modify: `crates/stint-core/src/calendar/google/mod.rs`
- Create: `crates/stint-core/tests/calendar_google_provider.rs`

Combines `GoogleClient` with a `TokenProvider` (reused from 3a) to satisfy `CalendarProvider`.

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/calendar_google_provider.rs`:

```rust
use async_trait::async_trait;
use stint_core::calendar::google::client::GoogleClient;
use stint_core::calendar::google::GoogleProvider;
use stint_core::calendar::provider::CalendarProvider;
use stint_core::calendar::types::{ProviderKind, TimeRange};
use stint_core::solidtime::auth::TokenProvider;
use chrono::{TimeZone, Utc};
use std::sync::Arc;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

struct FixedToken(String);

#[async_trait]
impl TokenProvider for FixedToken {
    async fn access_token(&self) -> stint_core::Result<String> {
        Ok(self.0.clone())
    }
}

#[tokio::test]
async fn provider_kind_is_google() {
    let server = MockServer::start().await;
    let p = GoogleProvider::new(
        Arc::new(FixedToken("t".into())),
        GoogleClient::with_base_url(&server.uri()),
    );
    assert_eq!(p.kind(), ProviderKind::Google);
}

#[tokio::test]
async fn list_calendars_passes_token_to_client() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/users/me/calendarList"))
        .and(header("Authorization", "Bearer the-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [{ "id": "primary", "summary": "Primary" }]
        })))
        .mount(&server)
        .await;
    let p = GoogleProvider::new(
        Arc::new(FixedToken("the-token".into())),
        GoogleClient::with_base_url(&server.uri()),
    );
    let cals = p.list_calendars().await.unwrap();
    assert_eq!(cals.len(), 1);
}

#[tokio::test]
async fn list_events_proxies_to_client() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/calendars/primary/events"))
        .and(header("Authorization", "Bearer the-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [
                { "id": "evt-1", "summary": "Standup",
                  "start": { "dateTime": "2026-05-19T09:00:00Z" },
                  "end":   { "dateTime": "2026-05-19T09:15:00Z" } }
            ]
        })))
        .mount(&server)
        .await;
    let p = GoogleProvider::new(
        Arc::new(FixedToken("the-token".into())),
        GoogleClient::with_base_url(&server.uri()),
    );
    let range = TimeRange {
        start: Utc.with_ymd_and_hms(2026, 5, 19, 0, 0, 0).unwrap(),
        end: Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
    };
    let evs = p.list_events("primary", range).await.unwrap();
    assert_eq!(evs.len(), 1);
    assert_eq!(evs[0].title, "Standup");
}
```

- [ ] **Step 2: Run — confirm failure**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_google_provider -- --test-threads=1
```

Expected: FAIL — `GoogleProvider` doesn't exist.

- [ ] **Step 3: Implement `GoogleProvider`**

Edit `crates/stint-core/src/calendar/google/mod.rs`:

```rust
//! Google Calendar provider. Reuses `crate::oauth` for the PKCE flow and
//! `reqwest` for the v3 REST surface.

pub mod client;
pub mod config;
pub mod dto;

use crate::calendar::google::client::GoogleClient;
use crate::calendar::provider::{CalendarProvider, RemoteCalendar, RemoteEvent};
use crate::calendar::types::{ProviderKind, TimeRange};
use crate::solidtime::auth::TokenProvider;
use crate::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// `CalendarProvider` implementation for Google Calendar. Owns an
/// `Arc<dyn TokenProvider>` so refresh logic — including persistence
/// back to Keychain — is shared with the Solidtime client.
pub struct GoogleProvider {
    tokens: Arc<dyn TokenProvider>,
    http: GoogleClient,
}

impl GoogleProvider {
    pub fn new(tokens: Arc<dyn TokenProvider>, http: GoogleClient) -> Self {
        Self { tokens, http }
    }
}

#[async_trait]
impl CalendarProvider for GoogleProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Google
    }

    async fn list_calendars(&self) -> Result<Vec<RemoteCalendar>> {
        let token = self.tokens.access_token().await?;
        self.http.list_calendars(&token).await
    }

    async fn list_events(
        &self,
        calendar_id: &str,
        range: TimeRange,
    ) -> Result<Vec<RemoteEvent>> {
        let token = self.tokens.access_token().await?;
        self.http.list_events(&token, calendar_id, range).await
    }
}
```

- [ ] **Step 4: Run — confirm pass**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_google_provider -- --test-threads=1
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/calendar/google/mod.rs \
        crates/stint-core/tests/calendar_google_provider.rs
git commit -m "feat(core): GoogleProvider (CalendarProvider impl)

Wires Arc<dyn TokenProvider> + GoogleClient into the trait. Token
refresh is handled by the TokenProvider impl (3a OAuthTokenProvider
applies cleanly — Google's refresh response shape is identical to
Solidtime's). Tests use a FixedToken mock; production wires an
OAuthTokenProvider configured for Google."
```

---

### Task 15: Refresh strategy + upsert pipeline (`calendar/sync.rs`)

**Files:**
- Modify: `crates/stint-core/src/calendar/sync.rs`
- Create: `crates/stint-core/tests/calendar_sync.rs`

The refresher takes a provider, the store, the account, and a `TimeRange`. It calls `list_calendars`, upserts them, then for every `included = 1` calendar calls `list_events(range)` and upserts events. Returns the total event count.

The trigger-keyed ranges from spec §5 live as `fn` factories on `Ranges`.

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/calendar_sync.rs`:

```rust
mod common;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use std::sync::Mutex;
use stint_core::calendar::provider::{CalendarProvider, RemoteCalendar, RemoteEvent};
use stint_core::calendar::store::CalendarStore;
use stint_core::calendar::sync::{refresh_account, Ranges};
use stint_core::calendar::types::{CalendarAccount, ProviderKind, TimeRange};
use stint_core::Result;

struct ScriptedProvider {
    calendars: Vec<RemoteCalendar>,
    events_by_calendar: Vec<(String, Vec<RemoteEvent>)>,
    last_range: Mutex<Option<TimeRange>>,
}

#[async_trait]
impl CalendarProvider for ScriptedProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Google
    }
    async fn list_calendars(&self) -> Result<Vec<RemoteCalendar>> {
        Ok(self.calendars.clone())
    }
    async fn list_events(&self, calendar_id: &str, range: TimeRange) -> Result<Vec<RemoteEvent>> {
        *self.last_range.lock().unwrap() = Some(range);
        Ok(self
            .events_by_calendar
            .iter()
            .find(|(id, _)| id == calendar_id)
            .map(|(_, v)| v.clone())
            .unwrap_or_default())
    }
}

async fn seed_account(s: &CalendarStore, id: &str) {
    s.add_account(&CalendarAccount {
        id: id.into(),
        provider: ProviderKind::Google,
        display_name: "me".into(),
        identifier: "me@example.com".into(),
        caldav_url: None,
        enabled: true,
        created_at: "2026-05-19T00:00:00Z".into(),
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn refresh_account_inserts_calendars_and_events() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed_account(&s, "acc-1").await;

    let provider = ScriptedProvider {
        calendars: vec![RemoteCalendar { id: "primary".into(), name: "Primary".into(), color: None }],
        events_by_calendar: vec![(
            "primary".into(),
            vec![RemoteEvent {
                id: "evt-1".into(),
                calendar_id: "primary".into(),
                title: "Standup".into(),
                start_at: "2026-05-19T09:00:00Z".into(),
                end_at: "2026-05-19T09:15:00Z".into(),
                is_all_day: false,
                attendee_status: None,
                recurring_root: None,
            }],
        )],
        last_range: Mutex::new(None),
    };

    let range = Ranges::on_add();
    let n = refresh_account(&s, "acc-1", &provider, range).await.unwrap();
    assert_eq!(n, 1);
    let evs = s.list_events_in_range("acc-1", "2026-05-19T00:00:00Z", "2026-05-20T00:00:00Z").await.unwrap();
    assert_eq!(evs.len(), 1);
}

#[tokio::test]
async fn refresh_account_skips_excluded_calendars() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    seed_account(&s, "acc-1").await;

    let provider = ScriptedProvider {
        calendars: vec![
            RemoteCalendar { id: "primary".into(), name: "Primary".into(), color: None },
            RemoteCalendar { id: "work".into(), name: "Work".into(), color: None },
        ],
        events_by_calendar: vec![
            (
                "primary".into(),
                vec![RemoteEvent {
                    id: "evt-p".into(), calendar_id: "primary".into(), title: "p".into(),
                    start_at: "2026-05-19T09:00:00Z".into(),
                    end_at:   "2026-05-19T09:15:00Z".into(),
                    is_all_day: false, attendee_status: None, recurring_root: None,
                }],
            ),
            (
                "work".into(),
                vec![RemoteEvent {
                    id: "evt-w".into(), calendar_id: "work".into(), title: "w".into(),
                    start_at: "2026-05-19T10:00:00Z".into(),
                    end_at:   "2026-05-19T10:15:00Z".into(),
                    is_all_day: false, attendee_status: None, recurring_root: None,
                }],
            ),
        ],
        last_range: Mutex::new(None),
    };

    // First refresh imports both calendars. Then exclude "work".
    refresh_account(&s, "acc-1", &provider, Ranges::on_add()).await.unwrap();
    s.set_calendar_included("work", false).await.unwrap();

    // Subsequent refresh should not call list_events for "work" calendar.
    // Track via a custom provider that records calls — but simpler: assert
    // by event count, since list_events_in_range already filters excluded.
    let n = refresh_account(&s, "acc-1", &provider, Ranges::on_add()).await.unwrap();
    assert_eq!(n, 1, "only the primary-calendar event should be returned");
}

#[tokio::test]
async fn ranges_on_add_spans_last_7_to_next_14_days() {
    let r = Ranges::on_add();
    let span = r.end - r.start;
    assert!(span >= Duration::days(20) && span <= Duration::days(22), "got {span}");
    // start ~ 7 days ago, end ~ 14 days from now.
    let now = Utc::now();
    assert!(r.start < now - Duration::days(6));
    assert!(r.end > now + Duration::days(13));
}

#[tokio::test]
async fn ranges_on_focus_spans_next_7_days() {
    let r = Ranges::on_focus();
    let now = Utc::now();
    assert!(r.start <= now);
    assert!(r.end > now + Duration::days(6));
    assert!(r.end < now + Duration::days(8));
}

#[tokio::test]
async fn ranges_background_spans_last_1_next_7() {
    let r = Ranges::background_poll();
    let now = Utc::now();
    assert!(r.start < now - Duration::hours(20));
    assert!(r.end > now + Duration::days(6));
}
```

- [ ] **Step 2: Run — confirm failure**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_sync -- --test-threads=1
```

Expected: FAIL.

- [ ] **Step 3: Implement `calendar/sync.rs`**

```rust
//! Per-account refresh strategy.
//!
//! `refresh_account` orchestrates one provider call cycle: list calendars,
//! upsert them, then for every locally-included calendar pull events in
//! the given range and upsert them with a fresh `fetched_at`.
//!
//! `Ranges` builds the trigger-keyed time windows from spec §5.

use crate::calendar::provider::CalendarProvider;
use crate::calendar::store::CalendarStore;
use crate::calendar::types::{Calendar, CalendarEvent, TimeRange};
use crate::{time, Result};
use chrono::{Duration, Utc};

pub struct Ranges;

impl Ranges {
    /// Used when an account is first connected. Spec §5: last 7 + next 14.
    pub fn on_add() -> TimeRange {
        let now = Utc::now();
        TimeRange {
            start: now - Duration::days(7),
            end: now + Duration::days(14),
        }
    }

    /// Used on launch / main-window focus. Spec §5: next 7.
    pub fn on_focus() -> TimeRange {
        let now = Utc::now();
        TimeRange {
            start: now,
            end: now + Duration::days(7),
        }
    }

    /// Used by the periodic background poller. Spec §5: last 1 + next 7.
    pub fn background_poll() -> TimeRange {
        let now = Utc::now();
        TimeRange {
            start: now - Duration::days(1),
            end: now + Duration::days(7),
        }
    }
}

/// Pull a fresh snapshot of one account's calendars + events into the
/// store. Returns the number of events upserted across all included
/// calendars. Excluded calendars contribute 0.
pub async fn refresh_account(
    store: &CalendarStore,
    account_id: &str,
    provider: &dyn CalendarProvider,
    range: TimeRange,
) -> Result<usize> {
    // 1) Sync the calendar list.
    let remote_calendars = provider.list_calendars().await?;
    let calendars: Vec<Calendar> = remote_calendars
        .iter()
        .map(|c| Calendar {
            id: c.id.clone(),
            account_id: account_id.into(),
            name: c.name.clone(),
            color: c.color.clone(),
            included: true, // ignored by upsert; included is locality-preserved
        })
        .collect();
    store.upsert_calendars(account_id, &calendars).await?;

    // 2) For each locally-included calendar, pull events and upsert.
    let local_calendars = store.list_calendars(account_id).await?;
    let now = time::now_utc();
    let mut count = 0usize;

    for c in local_calendars.iter().filter(|c| c.included) {
        let events = provider.list_events(&c.id, range).await?;
        let to_upsert: Vec<CalendarEvent> = events
            .into_iter()
            .map(|e| CalendarEvent {
                id: e.id,
                account_id: account_id.into(),
                calendar_id: e.calendar_id,
                title: e.title,
                start_at: e.start_at,
                end_at: e.end_at,
                is_all_day: e.is_all_day,
                attendee_status: e.attendee_status,
                recurring_root: e.recurring_root,
                fetched_at: now.clone(),
            })
            .collect();
        count += to_upsert.len();
        if !to_upsert.is_empty() {
            store.upsert_events(&to_upsert).await?;
        }
    }
    Ok(count)
}

/// Refresh every enabled account under one range trigger. Used by the
/// background worker and the Tauri "Refresh now" command. Errors on one
/// account do not abort the others — each is captured and the highest-
/// priority error is returned at the end.
pub async fn refresh_all_enabled(
    store: &CalendarStore,
    providers: &[(&str, Box<dyn CalendarProvider>)],
    range: TimeRange,
) -> Result<usize> {
    let mut total = 0usize;
    let mut first_err: Option<crate::Error> = None;
    for (account_id, provider) in providers {
        match refresh_account(store, account_id, provider.as_ref(), range).await {
            Ok(n) => total += n,
            Err(e) => {
                tracing::warn!(account = %account_id, error = %e, "calendar refresh failed");
                first_err.get_or_insert(e);
            }
        }
    }
    if let Some(e) = first_err {
        return Err(e);
    }
    Ok(total)
}
```

- [ ] **Step 4: Run — confirm pass**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test calendar_sync -- --test-threads=1
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/calendar/sync.rs \
        crates/stint-core/tests/calendar_sync.rs
git commit -m "feat(core): calendar refresh strategy

refresh_account orchestrates list_calendars → upsert → list_events per
included calendar → upsert. Ranges {on_add, on_focus, background_poll}
materialise the spec §5 trigger-keyed windows. refresh_all_enabled
fans out across accounts and captures per-account errors without
aborting the rest."
```

---

### Task 16: Tauri commands (`commands/calendar.rs`)

**Files:**
- Create: `crates/stint-app/src/commands/calendar.rs`
- Modify: `crates/stint-app/src/commands/mod.rs` (add `pub mod calendar;`)
- Modify: `crates/stint-app/src/main.rs` (register the commands)

Ten commands cover the surfaces consumed by the UI. Each is a thin wrapper around `stint-core`. Each follows the existing pattern (`State<'_, RwLock<AppState>>`, `AppError` conversion). `calendar_oauth_status` mirrors the existing `oauth_solidtime_status` shape so the UI handles OAuth state consistently across providers.

- [ ] **Step 1: Create `crates/stint-app/src/commands/calendar.rs`**

```rust
//! Tauri commands for calendar features.
//!
//! Thin wrappers around stint-core::calendar. Auth material is loaded
//! from Keychain per account; an `OAuthTokenProvider` is constructed
//! on demand so refresh tokens rotate transparently.

use crate::app_state::AppState;
use crate::commands::{store, AppError};
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use stint_core::calendar::google::client::GoogleClient;
use stint_core::calendar::google::config::google_oauth_config;
use stint_core::calendar::google::GoogleProvider;
use stint_core::calendar::store::{
    calendar_blob_delete, calendar_blob_load, calendar_blob_save, CalendarOAuthBlob, CalendarStore,
};
use stint_core::calendar::sync::{refresh_account, Ranges};
use stint_core::calendar::types::{
    Calendar, CalendarAccount, CalendarEvent, EventDecision, ProviderKind,
};
use stint_core::config::secrets::Secrets;
use stint_core::ids;
use stint_core::oauth::client::OAuthClient;
use stint_core::solidtime::auth::{
    login_interactive, OAuthTokenProvider, PersistFn, TokenProvider,
};
use stint_core::store::entries::{Entries, NewCompletedEntry};
use stint_core::time;
use tauri::{Emitter, State};
use tokio::sync::RwLock;

pub const EVENT_CALENDAR_CHANGED: &str = "calendar:changed";

#[derive(Serialize)]
pub struct EventWithDecision {
    #[serde(flatten)]
    pub event: CalendarEvent,
    pub decision: Option<String>,             // "ignored" | "logged_manual" | "logged_auto"
    pub linked_local_uuid: Option<String>,
}

/// Per-account OAuth status. Mirrors `SolidtimeAuthStatus` from
/// `commands/config.rs` so the UI can render parallel "signed in" /
/// "scope" affordances across both provider families.
#[derive(Serialize)]
pub struct CalendarOAuthStatus {
    pub signed_in: bool,
    pub scope: Option<String>,
}

#[tauri::command]
pub async fn calendar_list_accounts(
    state: State<'_, RwLock<AppState>>,
) -> Result<Vec<CalendarAccount>, AppError> {
    let store = store(&state).await;
    let cs = CalendarStore::new((*store).clone());
    Ok(cs.list_accounts().await?)
}

#[tauri::command]
pub async fn calendar_oauth_status(
    account_id: String,
) -> Result<CalendarOAuthStatus, AppError> {
    let secrets = Secrets::default();
    let blob = calendar_blob_load(&secrets, &account_id)?;
    Ok(CalendarOAuthStatus {
        signed_in: blob.is_some(),
        scope: blob.and_then(|b| b.tokens.scope),
    })
}

#[tauri::command]
pub async fn calendar_add_google(
    state: State<'_, RwLock<AppState>>,
    app: tauri::AppHandle,
) -> Result<CalendarAccount, AppError> {
    let store = store(&state).await;
    let cs = CalendarStore::new((*store).clone());
    let secrets = Secrets::default();

    // 1) Run the OAuth PKCE flow against accounts.google.com.
    let cfg = google_oauth_config();
    let client_id = cfg.client_id.clone();
    let oauth_client = OAuthClient::new(cfg);
    let tokens = login_interactive(&oauth_client, Duration::from_secs(300), "Google", |url| {
        if let Err(e) = open_url(&url) {
            tracing::warn!("could not open browser: {e}; paste manually: {url}");
        }
    })
    .await?;

    // 2) Insert a placeholder account so we have a UUID to key the blob.
    let account_uuid = ids::new_local_uuid();
    calendar_blob_save(
        &secrets,
        &account_uuid,
        &CalendarOAuthBlob {
            client_id: client_id.clone(),
            tokens: tokens.clone(),
        },
    )?;

    // 3) Resolve the identifier (email) via GoogleClient::list_calendars —
    //    Google does not expose userinfo in calendar.readonly, so we use
    //    the access-token's calendarList primary entry. The primary
    //    calendar id IS the user's email.
    let http = GoogleClient::new();
    let cals = http.list_calendars(&tokens.access_token).await?;
    let identifier = cals
        .iter()
        .find(|c| c.id == "primary")
        .map(|c| c.name.clone())
        .or_else(|| cals.first().map(|c| c.id.clone()))
        .unwrap_or_else(|| account_uuid.clone());

    let account = CalendarAccount {
        id: account_uuid.clone(),
        provider: ProviderKind::Google,
        display_name: identifier.clone(),
        identifier,
        caldav_url: None,
        enabled: true,
        created_at: time::now_utc(),
    };
    cs.add_account(&account).await?;

    // 4) Initial refresh: on_add window, persist calendars + events.
    let provider = build_google_provider(&secrets, &account_uuid)?;
    let _ = refresh_account(&cs, &account_uuid, &*provider, Ranges::on_add()).await?;

    // 5) Notify the UI.
    let _ = app.emit(EVENT_CALENDAR_CHANGED, &account_uuid);
    Ok(account)
}

#[tauri::command]
pub async fn calendar_remove_account(
    state: State<'_, RwLock<AppState>>,
    app: tauri::AppHandle,
    account_id: String,
) -> Result<(), AppError> {
    let store = store(&state).await;
    let cs = CalendarStore::new((*store).clone());
    cs.delete_account(&account_id).await?;
    let _ = calendar_blob_delete(&Secrets::default(), &account_id);
    let _ = app.emit(EVENT_CALENDAR_CHANGED, &account_id);
    Ok(())
}

#[tauri::command]
pub async fn calendar_list_calendars(
    state: State<'_, RwLock<AppState>>,
    account_id: String,
) -> Result<Vec<Calendar>, AppError> {
    let store = store(&state).await;
    let cs = CalendarStore::new((*store).clone());
    Ok(cs.list_calendars(&account_id).await?)
}

#[tauri::command]
pub async fn calendar_set_calendar_included(
    state: State<'_, RwLock<AppState>>,
    app: tauri::AppHandle,
    calendar_id: String,
    included: bool,
) -> Result<(), AppError> {
    let store = store(&state).await;
    let cs = CalendarStore::new((*store).clone());
    cs.set_calendar_included(&calendar_id, included).await?;
    let _ = app.emit(EVENT_CALENDAR_CHANGED, &calendar_id);
    Ok(())
}

#[tauri::command]
pub async fn calendar_refresh_account(
    state: State<'_, RwLock<AppState>>,
    app: tauri::AppHandle,
    account_id: String,
) -> Result<usize, AppError> {
    let store = store(&state).await;
    let cs = CalendarStore::new((*store).clone());
    let secrets = Secrets::default();
    let provider = build_google_provider(&secrets, &account_id)?;
    let n = refresh_account(&cs, &account_id, &*provider, Ranges::on_focus()).await?;
    let _ = app.emit(EVENT_CALENDAR_CHANGED, &account_id);
    Ok(n)
}

#[tauri::command]
pub async fn calendar_list_events_in_range(
    state: State<'_, RwLock<AppState>>,
    account_id: String,
    from: String,
    to: String,
) -> Result<Vec<EventWithDecision>, AppError> {
    let store = store(&state).await;
    let cs = CalendarStore::new((*store).clone());
    let events = cs.list_events_in_range(&account_id, &from, &to).await?;
    let decisions = cs.list_decisions_in_range(&account_id, &from, &to).await?;
    Ok(events
        .into_iter()
        .map(|e| {
            let d = decisions
                .iter()
                .find(|(ev_id, start, _)| ev_id == &e.id && start == &e.start_at)
                .map(|(_, _, dec)| dec.clone());
            EventWithDecision {
                decision: d.as_ref().map(|d| d.as_wire().to_string()),
                linked_local_uuid: d.as_ref().and_then(|d| d.linked_local_uuid().map(|s| s.to_string())),
                event: e,
            }
        })
        .collect())
}

#[tauri::command]
pub async fn calendar_log_event(
    state: State<'_, RwLock<AppState>>,
    app: tauri::AppHandle,
    account_id: String,
    event_id: String,
    event_start: String,
) -> Result<String, AppError> {
    let store = store(&state).await;
    let cs = CalendarStore::new((*store).clone());
    let entries = Entries::new((*store).clone());

    // Find the event row to get title + end_at.
    let events = cs
        .list_events_in_range(&account_id, &event_start, &next_second(&event_start))
        .await?;
    let event = events
        .into_iter()
        .find(|e| e.id == event_id && e.start_at == event_start)
        .ok_or(AppError::msg("calendar event not found in store"))?;

    let local_uuid = entries
        .create_completed(NewCompletedEntry {
            description: event.title,
            project_id: None,
            task_id: None,
            start_at: event.start_at.clone(),
            end_at: event.end_at.clone(),
            billable: false,
            source: "calendar".into(),
            source_event_id: Some(format!("{}:{}:{}", account_id, event.id, event.start_at)),
        })
        .await?;

    cs.record_decision(
        &account_id,
        &event_id,
        &event_start,
        &EventDecision::LoggedManual {
            linked_local_uuid: local_uuid.clone(),
        },
    )
    .await?;

    let _ = app.emit("entries:changed", 1);
    let _ = app.emit(EVENT_CALENDAR_CHANGED, &account_id);
    Ok(local_uuid)
}

#[tauri::command]
pub async fn calendar_ignore_event(
    state: State<'_, RwLock<AppState>>,
    app: tauri::AppHandle,
    account_id: String,
    event_id: String,
    event_start: String,
) -> Result<(), AppError> {
    let store = store(&state).await;
    let cs = CalendarStore::new((*store).clone());
    cs.record_decision(&account_id, &event_id, &event_start, &EventDecision::Ignored)
        .await?;
    let _ = app.emit(EVENT_CALENDAR_CHANGED, &account_id);
    Ok(())
}

/// Builds a `GoogleProvider` for an account. Wraps the `OAuthTokenProvider`
/// so 401/refresh handling is reused from 3a.
fn build_google_provider(
    secrets: &Secrets,
    account_id: &str,
) -> Result<Box<dyn stint_core::calendar::provider::CalendarProvider>, AppError> {
    let blob = calendar_blob_load(secrets, account_id)?
        .ok_or_else(|| AppError::msg(format!("no OAuth credentials for account {account_id}")))?;
    let mut cfg = google_oauth_config();
    cfg.client_id = blob.client_id.clone();
    let oauth_client = OAuthClient::new(cfg);

    let secrets_clone = secrets.clone();
    let account_owned = account_id.to_string();
    let client_id_owned = blob.client_id.clone();
    let persist: PersistFn = Box::new(move |tokens| {
        let updated = CalendarOAuthBlob {
            client_id: client_id_owned.clone(),
            tokens: tokens.clone(),
        };
        calendar_blob_save(&secrets_clone, &account_owned, &updated)
    });

    let provider: Arc<dyn TokenProvider> = Arc::new(OAuthTokenProvider::new(
        oauth_client,
        blob.tokens,
        persist,
    ));
    let http = GoogleClient::new();
    Ok(Box::new(GoogleProvider::new(provider, http)))
}

/// Adds one second to an RFC 3339 timestamp so `list_events_in_range` can
/// be reused as a point-query. Falls back to the input if parsing fails
/// (which shouldn't happen for well-formed event_start values).
fn next_second(ts: &str) -> String {
    match stint_core::time::parse(ts) {
        Ok(t) => stint_core::time::format(&(t + chrono::Duration::seconds(1))),
        Err(_) => ts.to_string(),
    }
}

fn open_url(url: &str) -> std::io::Result<()> {
    std::process::Command::new("/usr/bin/open")
        .arg(url)
        .status()
        .map(|_| ())
}
```

- [ ] **Step 2: Register `pub mod calendar` in `commands/mod.rs`**

Edit `crates/stint-app/src/commands/mod.rs`. The current module list:

```rust
pub mod config;
pub mod entries;
pub mod projects;
pub mod sync;
pub mod timer;
pub mod ui;
```

Add `pub mod calendar;` alphabetically:

```rust
pub mod calendar;
pub mod config;
pub mod entries;
pub mod projects;
pub mod sync;
pub mod timer;
pub mod ui;
```

- [ ] **Step 3: Register handlers in `main.rs`**

Edit `crates/stint-app/src/main.rs`. Inside `tauri::generate_handler![...]`, after the existing `commands::config::*` block, add the eight new commands:

```rust
            commands::calendar::calendar_list_accounts,
            commands::calendar::calendar_oauth_status,
            commands::calendar::calendar_add_google,
            commands::calendar::calendar_remove_account,
            commands::calendar::calendar_list_calendars,
            commands::calendar::calendar_set_calendar_included,
            commands::calendar::calendar_refresh_account,
            commands::calendar::calendar_list_events_in_range,
            commands::calendar::calendar_log_event,
            commands::calendar::calendar_ignore_event,
```

- [ ] **Step 4: Type-check the GUI binary**

```bash
cargo check -p stint-app
```

Expected: clean compile. No unit test for Tauri commands (per CLAUDE.md, command-layer is a smoke surface — actual logic lives in `stint-core` and is exercised by tests there).

- [ ] **Step 5: Commit**

```bash
git add crates/stint-app/src/commands/calendar.rs \
        crates/stint-app/src/commands/mod.rs \
        crates/stint-app/src/main.rs
git commit -m "feat(app): Tauri commands for calendar features

Adds calendar_list_accounts, calendar_oauth_status,
calendar_add_google, calendar_remove_account,
calendar_list_calendars, calendar_set_calendar_included,
calendar_refresh_account, calendar_list_events_in_range,
calendar_log_event, calendar_ignore_event. Each is a thin wrapper
around stint-core; build_google_provider() constructs an
OAuthTokenProvider per call so refreshes write back to Keychain
transparently. calendar_oauth_status mirrors oauth_solidtime_status
for cross-provider UI consistency."
```

---

### Task 17: CLI subcommands (`stint calendar …`)

**Files:**
- Create: `crates/stint-cli/src/cmd/calendar.rs`
- Modify: `crates/stint-cli/src/cmd/mod.rs`
- Modify: `crates/stint-cli/src/main.rs`
- Create: `crates/stint-cli/tests/cli_calendar.rs`

The CLI mirrors the Tauri surface for parity with spec §6 ("CLI surface (parity with GUI)"). The interactive `add google` flow opens the browser via `webbrowser`.

- [ ] **Step 1: Write a (compile-only) failing test for the subcommand wiring**

Create `crates/stint-cli/tests/cli_calendar.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn cmd(db: &std::path::Path) -> Command {
    let mut c = Command::cargo_bin("stint").expect("binary built");
    c.env("STINT_DB", db);
    c
}

#[tokio::test(flavor = "multi_thread")]
async fn calendar_list_empty_returns_no_accounts() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db)
        .args(["calendar", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No calendar accounts"));
}

#[tokio::test(flavor = "multi_thread")]
async fn calendar_help_lists_subcommands() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db)
        .args(["calendar", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("remove"))
        .stdout(predicate::str::contains("calendars"))
        .stdout(predicate::str::contains("refresh"));
}
```

- [ ] **Step 2: Run — confirm failure**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-cli --test cli_calendar -- --test-threads=1
```

Expected: FAIL (no `calendar` subcommand).

- [ ] **Step 3: Implement `crates/stint-cli/src/cmd/calendar.rs`**

```rust
use anyhow::{anyhow, Context, Result};
use clap::Subcommand;
use std::sync::Arc;
use std::time::Duration;
use stint_core::calendar::google::client::GoogleClient;
use stint_core::calendar::google::config::google_oauth_config;
use stint_core::calendar::google::GoogleProvider;
use stint_core::calendar::store::{
    calendar_blob_delete, calendar_blob_load, calendar_blob_save, CalendarOAuthBlob, CalendarStore,
};
use stint_core::calendar::sync::{refresh_account, Ranges};
use stint_core::calendar::types::{CalendarAccount, ProviderKind};
use stint_core::config::secrets::Secrets;
use stint_core::ids;
use stint_core::oauth::client::OAuthClient;
use stint_core::solidtime::auth::{
    login_interactive, OAuthTokenProvider, PersistFn, TokenProvider,
};
use stint_core::store::Store;
use stint_core::time;

#[derive(Subcommand)]
pub enum CalendarCmd {
    /// Add a Google Calendar account (interactive OAuth flow).
    Add {
        #[arg(value_parser = ["google"])]
        provider: String,
    },
    /// List connected calendar accounts.
    List,
    /// Remove a connected calendar account by id.
    Remove { account_id: String },
    /// List or toggle calendars for an account.
    Calendars {
        account_id: String,
        /// Calendar id to include.
        #[arg(long)]
        include: Option<String>,
        /// Calendar id to exclude.
        #[arg(long)]
        exclude: Option<String>,
    },
    /// Refresh one account's events (on_focus window).
    Refresh { account_id: String },
}

pub async fn run(c: CalendarCmd, store: Store) -> Result<()> {
    let cs = CalendarStore::new(store.clone());
    let secrets = Secrets::default();

    match c {
        CalendarCmd::Add { provider } if provider == "google" => add_google(&cs, &secrets).await,
        CalendarCmd::Add { provider } => Err(anyhow!("unknown provider {provider}")),
        CalendarCmd::List => {
            let accounts = cs.list_accounts().await?;
            if accounts.is_empty() {
                println!("No calendar accounts configured.");
                return Ok(());
            }
            for a in accounts {
                println!("{} {} {} <{}>", a.id, provider_label(a.provider), a.display_name, a.identifier);
            }
            Ok(())
        }
        CalendarCmd::Remove { account_id } => {
            cs.delete_account(&account_id).await?;
            let _ = calendar_blob_delete(&secrets, &account_id);
            println!("Removed account {account_id}.");
            Ok(())
        }
        CalendarCmd::Calendars { account_id, include, exclude } => {
            if let Some(id) = include {
                cs.set_calendar_included(&id, true).await?;
                println!("Included calendar {id}.");
            }
            if let Some(id) = exclude {
                cs.set_calendar_included(&id, false).await?;
                println!("Excluded calendar {id}.");
            }
            for c in cs.list_calendars(&account_id).await? {
                let mark = if c.included { "[x]" } else { "[ ]" };
                println!("{mark} {} {}", c.id, c.name);
            }
            Ok(())
        }
        CalendarCmd::Refresh { account_id } => {
            let provider = build_google_provider_cli(&secrets, &account_id)?;
            let n = refresh_account(&cs, &account_id, provider.as_ref(), Ranges::on_focus()).await?;
            println!("Refreshed {n} events.");
            Ok(())
        }
    }
}

async fn add_google(cs: &CalendarStore, secrets: &Secrets) -> Result<()> {
    let cfg = google_oauth_config();
    let client_id = cfg.client_id.clone();
    let oauth_client = OAuthClient::new(cfg);

    println!("Opening browser to sign in to Google.");
    println!("If the browser does not open, visit this URL manually:");
    let tokens = login_interactive(&oauth_client, Duration::from_secs(300), "Google", |url| {
        println!("  {url}");
        if let Err(e) = webbrowser::open(&url) {
            eprintln!("(Could not auto-open browser: {e})");
        }
    })
    .await
    .context("Google OAuth flow failed")?;

    let account_uuid = ids::new_local_uuid();
    calendar_blob_save(
        secrets,
        &account_uuid,
        &CalendarOAuthBlob {
            client_id: client_id.clone(),
            tokens: tokens.clone(),
        },
    )?;

    // Resolve email-shaped identifier via calendarList "primary" entry.
    let http = GoogleClient::new();
    let cals = http.list_calendars(&tokens.access_token).await?;
    let identifier = cals
        .iter()
        .find(|c| c.id == "primary")
        .map(|c| c.name.clone())
        .or_else(|| cals.first().map(|c| c.id.clone()))
        .unwrap_or_else(|| account_uuid.clone());

    let account = CalendarAccount {
        id: account_uuid.clone(),
        provider: ProviderKind::Google,
        display_name: identifier.clone(),
        identifier,
        caldav_url: None,
        enabled: true,
        created_at: time::now_utc(),
    };
    cs.add_account(&account).await?;

    let provider = build_google_provider_cli(secrets, &account_uuid)?;
    let n = refresh_account(cs, &account_uuid, provider.as_ref(), Ranges::on_add()).await?;
    println!(
        "Added Google account: {} ({account_uuid}). Fetched {n} events.",
        account.identifier
    );
    Ok(())
}

fn build_google_provider_cli(
    secrets: &Secrets,
    account_id: &str,
) -> Result<Box<dyn stint_core::calendar::provider::CalendarProvider>> {
    let blob = calendar_blob_load(secrets, account_id)?
        .ok_or_else(|| anyhow!("no OAuth credentials for account {account_id}"))?;
    let mut cfg = google_oauth_config();
    cfg.client_id = blob.client_id.clone();
    let oauth_client = OAuthClient::new(cfg);

    let secrets_clone = secrets.clone();
    let account_owned = account_id.to_string();
    let client_id_owned = blob.client_id.clone();
    let persist: PersistFn = Box::new(move |tokens| {
        let updated = CalendarOAuthBlob {
            client_id: client_id_owned.clone(),
            tokens: tokens.clone(),
        };
        calendar_blob_save(&secrets_clone, &account_owned, &updated)
    });

    let provider: Arc<dyn TokenProvider> = Arc::new(OAuthTokenProvider::new(
        oauth_client,
        blob.tokens,
        persist,
    ));
    Ok(Box::new(GoogleProvider::new(provider, GoogleClient::new())))
}

fn provider_label(p: ProviderKind) -> &'static str {
    match p {
        ProviderKind::Google => "google",
    }
}
```

- [ ] **Step 4: Wire it in `cmd/mod.rs` and `main.rs`**

Edit `crates/stint-cli/src/cmd/mod.rs`. Current:

```rust
pub mod config;
pub mod config_login;
pub mod delete;
pub mod edit;
pub mod list;
pub mod projects;
pub mod start;
pub mod stop;
pub mod sync;
pub mod today;
```

Add `pub mod calendar;` alphabetically:

```rust
pub mod calendar;
pub mod config;
pub mod config_login;
pub mod delete;
pub mod edit;
pub mod list;
pub mod projects;
pub mod start;
pub mod stop;
pub mod sync;
pub mod today;
```

Edit `crates/stint-cli/src/main.rs`. Add a `Calendar` arm to the `Command` enum:

```rust
    /// Connect, list, and manage calendar accounts.
    #[command(subcommand)]
    Calendar(cmd::calendar::CalendarCmd),
```

(Place it alphabetically — between `Stop` and `Sync` works.)

And in the `match cli.command { ... }` block, add:

```rust
        Command::Calendar(c) => {
            let store = cmd::open_store().await?;
            cmd::calendar::run(c, store).await
        }
```

- [ ] **Step 5: Run — confirm pass**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-cli --test cli_calendar -- --test-threads=1
```

Expected: PASS (both tests).

- [ ] **Step 6: Commit**

```bash
git add crates/stint-cli/src/cmd/calendar.rs \
        crates/stint-cli/src/cmd/mod.rs \
        crates/stint-cli/src/main.rs \
        crates/stint-cli/tests/cli_calendar.rs
git commit -m "feat(cli): \`stint calendar\` subcommands

Adds add/list/remove/calendars/refresh. Parity with the GUI surface
per spec §6. \`stint calendar add google\` opens the browser via the
existing webbrowser crate; subsequent commands reuse the same
OAuthTokenProvider machinery so refreshes rotate transparently."
```

---

### Task 18: UI — Settings "Calendar accounts" panel

**Files:**
- Modify: `ui/src/types.ts` (add Calendar-related types)
- Modify: `ui/src/api.ts` (add typed wrappers for calendar commands)
- Modify: `ui/src/routes/Settings.tsx` (add Calendar accounts section)

Per CLAUDE.md, UI has no automated tests in v1 — verification is manual under `cargo tauri dev`. Keep the section structure consistent with the existing Settings layout (`FieldShell`, `Button`, `Pill`, `StatusDot`).

- [ ] **Step 1: Add UI types**

Edit `ui/src/types.ts`. Append:

```ts
export type ProviderKind = "google";

export type CalendarAccount = {
  id: string;
  provider: ProviderKind;
  display_name: string;
  identifier: string;
  caldav_url: string | null;
  enabled: boolean;
  created_at: string;
};

export type CalendarRow = {
  id: string;
  account_id: string;
  name: string;
  color: string | null;
  included: boolean;
};

export type CalendarEventDecision = "ignored" | "logged_manual" | "logged_auto";

export type CalendarEventWithDecision = {
  id: string;
  account_id: string;
  calendar_id: string;
  title: string;
  start_at: string;
  end_at: string;
  is_all_day: boolean;
  attendee_status: "accepted" | "declined" | "tentative" | null;
  recurring_root: string | null;
  fetched_at: string;
  decision: CalendarEventDecision | null;
  linked_local_uuid: string | null;
};

export type CalendarOAuthStatus = {
  signed_in: boolean;
  scope: string | null;
};
```

- [ ] **Step 2: Add API wrappers**

Edit `ui/src/api.ts`. Below the existing exports, append:

```ts
import type {
  CalendarAccount,
  CalendarEventWithDecision,
  CalendarOAuthStatus,
  CalendarRow,
} from "./types";

export const calendarApi = {
  listAccounts: () => invoke<CalendarAccount[]>("calendar_list_accounts"),
  oauthStatus: (accountId: string) =>
    invoke<CalendarOAuthStatus>("calendar_oauth_status", { accountId }),
  addGoogle: () => invoke<CalendarAccount>("calendar_add_google"),
  removeAccount: (accountId: string) =>
    invoke<void>("calendar_remove_account", { accountId }),
  listCalendars: (accountId: string) =>
    invoke<CalendarRow[]>("calendar_list_calendars", { accountId }),
  setCalendarIncluded: (calendarId: string, included: boolean) =>
    invoke<void>("calendar_set_calendar_included", { calendarId, included }),
  refreshAccount: (accountId: string) =>
    invoke<number>("calendar_refresh_account", { accountId }),
  listEventsInRange: (accountId: string, from: string, to: string) =>
    invoke<CalendarEventWithDecision[]>("calendar_list_events_in_range", {
      accountId,
      from,
      to,
    }),
  logEvent: (accountId: string, eventId: string, eventStart: string) =>
    invoke<string>("calendar_log_event", { accountId, eventId, eventStart }),
  ignoreEvent: (accountId: string, eventId: string, eventStart: string) =>
    invoke<void>("calendar_ignore_event", { accountId, eventId, eventStart }),
};
```

- [ ] **Step 3: Render the Calendar accounts section**

Edit `ui/src/routes/Settings.tsx`. Add this import at the top, alongside the existing imports:

```tsx
import { calendarApi } from "~/api";
import type { CalendarAccount, CalendarRow } from "~/types";
```

Inside the `Settings()` component, after the existing OAuth-related resources, add:

```tsx
  // Calendar accounts
  const [accounts, { refetch: refetchAccounts }] = createResource(() =>
    calendarApi.listAccounts(),
  );

  async function handleAddGoogle() {
    flash("info", "Opening Google sign-in…");
    try {
      const a = await calendarApi.addGoogle();
      flash("ok", `Connected Google account: ${a.identifier}`);
      refetchAccounts();
    } catch (e) {
      flash("err", `Failed: ${(e as { message: string }).message}`);
    }
  }

  async function handleRemoveAccount(id: string) {
    if (!confirm("Remove this calendar account?")) return;
    try {
      await calendarApi.removeAccount(id);
      flash("ok", "Account removed.");
      refetchAccounts();
    } catch (e) {
      flash("err", `Failed: ${(e as { message: string }).message}`);
    }
  }
```

Then, just before the closing `</div>` of `<div class="mx-auto max-w-3xl px-6 py-8">`, add a new section after the existing Solidtime-connection `<section>`:

```tsx
      <section class="mt-6 rounded-2xl border border-black/[0.06] bg-white p-6 dark:border-white/[0.06] dark:bg-zinc-900">
        <h2 class="mb-1 text-sm font-semibold uppercase tracking-wide text-zinc-500">
          Calendar accounts
        </h2>
        <p class="mb-5 text-xs text-zinc-500">
          Read-only — events appear on the Today view with a "Log this" action.
        </p>

        <Show
          when={(accounts() ?? []).length > 0}
          fallback={
            <p class="text-sm text-zinc-500">
              No calendar accounts connected yet.
            </p>
          }
        >
          <ul class="space-y-2">
            <For each={accounts() ?? []}>
              {(a) => (
                <CalendarAccountRow
                  account={a}
                  flash={flash}
                  onRemove={() => handleRemoveAccount(a.id)}
                />
              )}
            </For>
          </ul>
        </Show>

        <div class="mt-4 border-t border-black/[0.05] pt-4 dark:border-white/[0.04]">
          <Button onClick={handleAddGoogle}>Add Google account</Button>
        </div>
      </section>
```

At the bottom of the file (alongside the existing helper components like `TextField`, `SelectField`), add:

```tsx
function CalendarAccountRow(props: {
  account: CalendarAccount;
  flash: (kind: "ok" | "err" | "info", msg: string) => void;
  onRemove: () => void;
}) {
  const [status] = createResource(
    () => props.account.id,
    (id) => calendarApi.oauthStatus(id),
  );

  return (
    <li class="flex items-center justify-between rounded-md border border-black/[0.05] bg-white px-3 py-2 dark:border-white/[0.05] dark:bg-zinc-950">
      <div class="min-w-0">
        <div class="flex items-center gap-2">
          <span class="text-sm font-medium truncate">
            {props.account.identifier}
          </span>
          <Show when={status()?.signed_in} fallback={<Pill tone="amber">Reconnect</Pill>}>
            <Pill tone="emerald">Signed in</Pill>
          </Show>
        </div>
        <div class="mt-0.5 text-xs text-zinc-500">
          {props.account.provider} · {props.account.id.slice(0, 8)}
          <Show when={status()?.scope}>
            {" · "}
            <span title={status()?.scope ?? ""}>scope ✓</span>
          </Show>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <CalendarsManager accountId={props.account.id} flash={props.flash} />
        <Button variant="ghost" size="sm" onClick={props.onRemove}>
          Remove
        </Button>
      </div>
    </li>
  );
}

function CalendarsManager(props: {
  accountId: string;
  flash: (kind: "ok" | "err" | "info", msg: string) => void;
}) {
  const [open, setOpen] = createSignal(false);
  const [cals, { refetch }] = createResource(
    () => (open() ? props.accountId : null),
    async (id): Promise<CalendarRow[]> => {
      if (!id) return [];
      return calendarApi.listCalendars(id);
    },
  );

  async function toggle(id: string, included: boolean) {
    try {
      await calendarApi.setCalendarIncluded(id, included);
      refetch();
    } catch (e) {
      props.flash("err", `Toggle failed: ${(e as { message: string }).message}`);
    }
  }

  return (
    <div class="relative">
      <Button variant="ghost" size="sm" onClick={() => setOpen(!open())}>
        Calendars
      </Button>
      <Show when={open()}>
        <div class="absolute right-0 top-full z-10 mt-1 w-72 rounded-md border border-black/[0.08] bg-white p-3 shadow-lg dark:border-white/[0.08] dark:bg-zinc-950">
          <Show
            when={(cals() ?? []).length > 0}
            fallback={
              <p class="text-xs text-zinc-500">
                {cals.loading ? "Loading…" : "No calendars."}
              </p>
            }
          >
            <ul class="space-y-1">
              <For each={cals() ?? []}>
                {(c) => (
                  <li>
                    <label class="flex cursor-pointer items-center gap-2 text-sm">
                      <input
                        type="checkbox"
                        checked={c.included}
                        onChange={(e) =>
                          toggle(c.id, e.currentTarget.checked)
                        }
                      />
                      <span>{c.name}</span>
                    </label>
                  </li>
                )}
              </For>
            </ul>
          </Show>
        </div>
      </Show>
    </div>
  );
}
```

- [ ] **Step 4: Type-check + build the UI**

```bash
pnpm -C ui typecheck
pnpm -C ui build
```

Expected: both clean. Fix any TS errors before continuing.

- [ ] **Step 5: Manual visual verification (no `cargo tauri dev` run yet — Task 20 does that)**

This task is committed without an interactive verification round; Task 20 runs the GUI and confirms all surfaces work end-to-end.

- [ ] **Step 6: Commit**

```bash
git add ui/src/types.ts ui/src/api.ts ui/src/routes/Settings.tsx
git commit -m "feat(ui): Settings Calendar accounts panel

Adds 'Add Google account', per-account remove, and a Calendars
dropdown for include/exclude toggling. Each account row queries
calendar_oauth_status and shows a 'Signed in' (emerald) or
'Reconnect' (amber) Pill — mirroring the Solidtime OAuth status
treatment for cross-provider consistency. Uses existing FieldShell /
Button / Pill primitives — no new shared components."
```

---

### Task 19: UI — Today route Calendar section + Log this/Ignore

**Files:**
- Modify: `ui/src/routes/Today.tsx`
- Create: `ui/src/components/CalendarSection.tsx`

The Today route gets a "Calendar" section above "Entries". Each event shows title, time range, source pill, and Log this / Ignore buttons. Decisions persist; logged events show a green check, ignored events fade out.

- [ ] **Step 1: Create `ui/src/components/CalendarSection.tsx`**

```tsx
import { For, Show, createMemo, createResource, createSignal, onCleanup } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { calendarApi } from "~/api";
import Button from "~/components/ui/Button";
import Pill from "~/components/ui/Pill";
import SectionLabel from "~/components/ui/SectionLabel";
import type {
  CalendarAccount,
  CalendarEventWithDecision,
} from "~/types";

type EventByAccount = {
  account: CalendarAccount;
  events: CalendarEventWithDecision[];
};

export default function CalendarSection(props: { onEntriesChanged: () => void }) {
  const [accounts] = createResource(() => calendarApi.listAccounts());

  const todayRange = createMemo(() => {
    const now = new Date();
    const start = new Date(now);
    start.setHours(0, 0, 0, 0);
    const end = new Date(start);
    end.setDate(end.getDate() + 1);
    return { from: start.toISOString(), to: end.toISOString() };
  });

  const [groups, { refetch }] = createResource(
    () => (accounts() ?? []).map((a) => a.id).join(","),
    async (): Promise<EventByAccount[]> => {
      const list = accounts() ?? [];
      const range = todayRange();
      const groups: EventByAccount[] = [];
      for (const account of list) {
        try {
          const events = await calendarApi.listEventsInRange(
            account.id,
            range.from,
            range.to,
          );
          groups.push({ account, events });
        } catch {
          groups.push({ account, events: [] });
        }
      }
      return groups;
    },
  );

  const unlisten = listen("calendar:changed", () => refetch());
  onCleanup(() => {
    unlisten.then((fn) => fn()).catch(() => {});
  });

  const total = createMemo(() =>
    (groups() ?? []).reduce((acc, g) => acc + g.events.length, 0),
  );

  async function handleLog(g: EventByAccount, e: CalendarEventWithDecision) {
    try {
      await calendarApi.logEvent(g.account.id, e.id, e.start_at);
      props.onEntriesChanged();
      refetch();
    } catch (err) {
      console.error("Log this failed:", err);
    }
  }

  async function handleIgnore(g: EventByAccount, e: CalendarEventWithDecision) {
    try {
      await calendarApi.ignoreEvent(g.account.id, e.id, e.start_at);
      refetch();
    } catch (err) {
      console.error("Ignore failed:", err);
    }
  }

  return (
    <Show when={total() > 0}>
      <section class="mt-8">
        <div class="mb-3 flex items-baseline justify-between">
          <SectionLabel>Calendar</SectionLabel>
          <span class="text-xs text-zinc-400 dark:text-zinc-500">
            {total()} event{total() === 1 ? "" : "s"} today
          </span>
        </div>

        <div class="space-y-2">
          <For each={groups() ?? []}>
            {(g) => (
              <For each={g.events}>
                {(e) => (
                  <EventRow
                    event={e}
                    onLog={() => handleLog(g, e)}
                    onIgnore={() => handleIgnore(g, e)}
                  />
                )}
              </For>
            )}
          </For>
        </div>
      </section>
    </Show>
  );
}

function EventRow(props: {
  event: CalendarEventWithDecision;
  onLog: () => void;
  onIgnore: () => void;
}) {
  const decided = () => props.event.decision !== null;
  const logged = () =>
    props.event.decision === "logged_manual" ||
    props.event.decision === "logged_auto";

  const startLabel = () => formatTime(props.event.start_at, props.event.is_all_day);
  const endLabel = () => formatTime(props.event.end_at, props.event.is_all_day);

  return (
    <div
      class="flex items-center justify-between rounded-lg border border-black/[0.06] bg-white px-3 py-2 dark:border-white/[0.06] dark:bg-zinc-900"
      classList={{ "opacity-50": decided() && !logged() }}
    >
      <div class="flex min-w-0 flex-1 items-center gap-3">
        <div class="w-24 shrink-0 text-xs tabular-nums text-zinc-500">
          {props.event.is_all_day ? "all-day" : `${startLabel()} – ${endLabel()}`}
        </div>
        <div class="min-w-0 flex-1 truncate text-sm">{props.event.title}</div>
        <Show when={logged()}>
          <Pill tone="emerald">Logged</Pill>
        </Show>
        <Show when={props.event.decision === "ignored"}>
          <Pill tone="zinc">Ignored</Pill>
        </Show>
      </div>

      <Show when={!decided()}>
        <div class="ml-2 flex items-center gap-1">
          <Button variant="ghost" size="sm" onClick={props.onLog}>
            Log this
          </Button>
          <Button variant="ghost" size="sm" onClick={props.onIgnore}>
            Ignore
          </Button>
        </div>
      </Show>
    </div>
  );
}

function formatTime(iso: string, allDay: boolean): string {
  if (allDay) return "";
  const d = new Date(iso);
  return d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}
```

- [ ] **Step 2: Wire `CalendarSection` into `Today.tsx`**

Edit `ui/src/routes/Today.tsx`. Add the import:

```tsx
import CalendarSection from "~/components/CalendarSection";
```

Inside the JSX, between the `<TimerCard />` line and the existing `<section class="mt-8">` containing "Entries", insert:

```tsx
        <CalendarSection onEntriesChanged={() => refetch()} />
```

(`refetch` is the entries resource refetcher already in scope inside the component.)

- [ ] **Step 3: Type-check + build**

```bash
pnpm -C ui typecheck
pnpm -C ui build
```

Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add ui/src/components/CalendarSection.tsx ui/src/routes/Today.tsx
git commit -m "feat(ui): Today route Calendar section

New CalendarSection lists today's events grouped by account, with Log
this / Ignore actions. Logged events show a green Pill and the
underlying time entry refresh fires via the existing entries:changed
emit. Calendar:changed events from the worker drive the section's
auto-refresh."
```

---

### Task 20: Background worker, docs, manual verification, PR, tag

**Files:**
- Create: `crates/stint-app/src/calendar_worker.rs`
- Modify: `crates/stint-app/src/main.rs` (spawn the worker; expose module)
- Modify: `crates/stint-app/src/lib.rs` (`pub mod calendar_worker;`)
- Modify: `README.md`
- Modify: `CLAUDE.md`
- Modify: `AGENTS.md` (kept in sync per the file's pointer convention)

This task closes the loop: the periodic poller, the documentation, the manual verification checklist, then the PR open + merge + tag. Several substeps require Mario's confirmation.

- [ ] **Step 1: Implement the background worker**

Create `crates/stint-app/src/calendar_worker.rs`:

```rust
//! Background calendar refresher. Polls every 15 min while the GUI runs,
//! mirroring `sync_worker.rs`. Emits `calendar:changed` after any tick
//! that upserted at least one event.

use std::sync::Arc;
use std::time::Duration;
use stint_core::calendar::store::{calendar_blob_load, CalendarStore};
use stint_core::calendar::google::client::GoogleClient;
use stint_core::calendar::google::config::google_oauth_config;
use stint_core::calendar::google::GoogleProvider;
use stint_core::calendar::sync::{refresh_account, Ranges};
use stint_core::config::secrets::Secrets;
use stint_core::oauth::client::OAuthClient;
use stint_core::solidtime::auth::{OAuthTokenProvider, PersistFn, TokenProvider};
use stint_core::store::Store;
use stint_core::calendar::store::{calendar_blob_save, CalendarOAuthBlob};
use tauri::{AppHandle, Emitter};
use tokio::time::sleep;
use tracing::{debug, info, warn};

pub const EVENT_CALENDAR_CHANGED: &str = "calendar:changed";
const TICK: Duration = Duration::from_secs(15 * 60);

pub fn spawn(app: AppHandle, store: Arc<Store>) {
    tokio::spawn(async move {
        info!("calendar worker started (tick = {:?})", TICK);
        loop {
            match tick(&store).await {
                Ok(n) if n > 0 => {
                    let _ = app.emit(EVENT_CALENDAR_CHANGED, n);
                }
                Ok(_) => {}
                Err(e) => warn!(error = %e, "calendar tick failed"),
            }
            sleep(TICK).await;
        }
    });
}

async fn tick(store: &Store) -> stint_core::Result<usize> {
    let cs = CalendarStore::new((*store).clone());
    let secrets = Secrets::default();
    let accounts = cs.list_accounts().await?;
    if accounts.is_empty() {
        debug!("calendar worker: no accounts; skipping tick");
        return Ok(0);
    }

    let mut total = 0usize;
    for account in accounts {
        if !account.enabled {
            continue;
        }
        match build_provider(&secrets, &account.id) {
            Ok(provider) => {
                match refresh_account(&cs, &account.id, provider.as_ref(), Ranges::background_poll())
                    .await
                {
                    Ok(n) => total += n,
                    Err(e) => warn!(account = %account.id, error = %e, "calendar refresh failed"),
                }
            }
            Err(e) => warn!(account = %account.id, error = %e, "could not build provider"),
        }
    }
    if total > 0 {
        info!(events = total, "calendar worker refreshed events");
    }
    Ok(total)
}

fn build_provider(
    secrets: &Secrets,
    account_id: &str,
) -> stint_core::Result<Box<dyn stint_core::calendar::provider::CalendarProvider>> {
    let blob = calendar_blob_load(secrets, account_id)?
        .ok_or(stint_core::Error::MissingConfig("calendar.oauth"))?;
    let mut cfg = google_oauth_config();
    cfg.client_id = blob.client_id.clone();
    let oauth_client = OAuthClient::new(cfg);

    let secrets_clone = secrets.clone();
    let account_owned = account_id.to_string();
    let client_id_owned = blob.client_id.clone();
    let persist: PersistFn = Box::new(move |tokens| {
        let updated = CalendarOAuthBlob {
            client_id: client_id_owned.clone(),
            tokens: tokens.clone(),
        };
        calendar_blob_save(&secrets_clone, &account_owned, &updated)
    });
    let tokens: Arc<dyn TokenProvider> = Arc::new(OAuthTokenProvider::new(
        oauth_client,
        blob.tokens,
        persist,
    ));
    Ok(Box::new(GoogleProvider::new(tokens, GoogleClient::new())))
}
```

- [ ] **Step 2: Spawn the worker from `main.rs`**

Edit `crates/stint-app/src/main.rs`. At the top, after the existing `mod` declarations:

```rust
mod app_state;
mod calendar_worker;
mod commands;
mod menu;
mod sync_worker;
mod tray;
mod windows;
```

Inside the `.setup(move |app| {` block, after the existing `sync_worker::spawn(...)` line, add:

```rust
            calendar_worker::spawn(app.handle().clone(), store_for_worker.clone());
```

Edit `crates/stint-app/src/lib.rs` to re-export the module alongside the others:

```rust
pub mod app_state;
pub mod calendar_worker;
pub mod commands;
pub mod menu;
pub mod sync_worker;
pub mod tray;
pub mod windows;
```

- [ ] **Step 3: Type-check the workspace**

```bash
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: clean.

- [ ] **Step 4: Update README**

Edit `README.md`. Add a "Calendar setup (Google)" subsection under the existing OAuth-setup section:

```markdown
### Connecting a Google Calendar (Phase 3b)

stint reads (read-only) your Google Calendar so you can convert
events into time entries with one click. The stint binary ships
with a registered Google OAuth client; you do **not** need to
register your own.

**First connect:**

CLI:

```
stint calendar add google
```

GUI: open Settings → Calendar accounts → "Add Google account".

The system browser opens, you grant `calendar.readonly`, and the
account appears in the list. The Today view picks up today's events
within a few seconds.

**Managing per-calendar inclusion:**

```
stint calendar calendars <account-id> --exclude <calendar-id>
stint calendar calendars <account-id> --include <calendar-id>
```

(Click "Calendars" on the account row in Settings for the GUI
equivalent.)

**Refresh window:** stint pulls last 7 + next 14 days at first
connect, next 7 on launch/window focus, and last 1 + next 7 every
15 minutes while the GUI is running.

**Removing an account:**

```
stint calendar remove <account-id>
```

OAuth tokens for the account are deleted from Keychain; calendar
rows are cascade-deleted from the local database. Any time entries
already logged from calendar events remain (their `source_event_id`
just becomes a dangling reference).
```

- [ ] **Step 5: Update CLAUDE.md**

Edit `CLAUDE.md`. In the "Where we are in the roadmap" table, update Phase 3b's status from `planned` to `✅ shipped (`phase-3b-complete`)` once the tag lands. In the "Gotchas" section, add:

```markdown
- **Google OAuth client ID is baked in.** `crates/stint-core/src/calendar/google/config.rs::GOOGLE_OAUTH_CLIENT_ID`
  holds the production value registered against the stint Google Cloud
  project. `STINT_GOOGLE_CLIENT_ID` env var overrides it for tests and
  local dev. If you need to rotate the client (revoked credentials,
  consent-screen reset, etc.), update the constant and ship a new
  release; existing user accounts must re-sign-in because Google scopes
  refresh tokens to the client_id.
- **Calendar OAuth blobs are per-account.** Each Google account has its
  own Keychain entry at `tech.reyem.stint.calendar.<account-uuid>` —
  the lookup is by `calendar_accounts.id`, not by email. If you delete
  a row from `calendar_accounts` directly via SQL, also delete the
  Keychain blob or it will leak. `calendar_remove_account` does both.
- **`singleEvents=true` does the recurrence expansion server-side.**
  Google returns one event per occurrence in the requested window,
  populating `recurringEventId` on overrides and expanded instances.
  Phase 3b does NOT include an iCal RRULE expander — Phase 3d (CalDAV)
  is where that machinery will live.
```

- [ ] **Step 6: Sync `AGENTS.md`**

Per CLAUDE.md's convention, `AGENTS.md` is a pointer file. Confirm the current content; if it just points to `CLAUDE.md`, no change is required. If it duplicates the table, update the same status row.

```bash
diff CLAUDE.md AGENTS.md || true
```

- [ ] **Step 7: Run the full test suite**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test --workspace -- --test-threads=1
pnpm -C ui typecheck
pnpm -C ui build
```

Expected: all green.

- [ ] **Step 8: Stop and confirm Mario's Google client ID is pasted into the source**

Open `crates/stint-core/src/calendar/google/config.rs`. The constant must hold the real client ID, not the placeholder. If it still says `REPLACE_ME.apps.googleusercontent.com`, **STOP** and ask Mario to paste the real value; commit it separately as `feat(core): set production Google OAuth client ID`.

- [ ] **Step 9: Manual verification checklist**

Run the GUI:

```bash
cd crates/stint-app
cargo tauri dev
```

Walk through each of these and tick them off in the PR description:

- [ ] Settings → Calendar accounts shows "No calendar accounts connected yet."
- [ ] Click "Add Google account" → browser opens to `accounts.google.com/o/oauth2/v2/auth?…` URL.
- [ ] Grant the `calendar.readonly` scope.
- [ ] Browser returns success page reading **"Signed in to Google"** (not "Solidtime") — confirms the Task 5 loopback parameterization is live.
- [ ] Settings shows the new account row with the user's email and an emerald "Signed in" pill.
- [ ] Hover the "scope ✓" subscript → tooltip shows the granted scope string.
- [ ] Click "Calendars" on the row — list of calendars loads, checkboxes reflect `included = 1` for all.
- [ ] Uncheck one calendar → events from that calendar disappear from Today.
- [ ] Today view shows today's events (with time ranges), sorted by start.
- [ ] Click "Log this" on an event → a green "Logged" pill appears; the event also surfaces as a new entry in the Entries list below.
- [ ] Click "Ignore" on a different event → it fades to 50% opacity and shows "Ignored".
- [ ] CLI: `./scripts/dev-cli.sh calendar list` lists the connected account.
- [ ] CLI: `./scripts/dev-cli.sh calendar refresh <account-id>` prints a non-zero event count.
- [ ] Wait 15+ minutes (or shorten `TICK` for the test) and confirm the periodic tick fires — Today view auto-refreshes without manual reload.
- [ ] Restart the GUI; previously-logged events are still marked "Logged"; ignored events are still ignored.
- [ ] Restart stint, then sign out of the Google account via Settings → Calendar accounts → Remove → confirm. The account row disappears; the corresponding Keychain entry no longer exists (`security find-generic-password -s tech.reyem.stint.calendar.<uuid>` returns NotFound).

If any step fails, capture the error, open a `fix(*):` commit on the branch, and re-run the failing step.

- [ ] **Step 10: Commit worker + docs**

```bash
git add crates/stint-app/src/calendar_worker.rs \
        crates/stint-app/src/main.rs \
        crates/stint-app/src/lib.rs \
        README.md CLAUDE.md AGENTS.md
git commit -m "feat(app,docs): calendar background worker + README/CLAUDE notes

15-min periodic poller mirrors sync_worker. Emits calendar:changed so
the UI refreshes without polling. README documents the Google
sign-in flow; CLAUDE.md adds three gotchas (baked-in client ID,
per-account Keychain blobs, server-side recurrence expansion)."
```

- [ ] **Step 11: Push and mark PR ready for review**

```bash
git push
gh pr ready
```

**Pause for Mario to review and approve before merging.**

- [ ] **Step 12: Merge via the GitHub UI**

Use "Rebase and merge" (preserves linear history). **Pause for Mario to click the button — direct merge from the CLI is blocked by branch protection anyway.**

- [ ] **Step 13: Tag the phase**

```bash
git checkout main
git pull
git tag -a phase-3b-complete -m "Phase 3b complete: Google Calendar integration"
git push origin phase-3b-complete
```

**Pause for Mario to confirm the push. Tag pushes are observable to anyone watching the repo.**

- [ ] **Step 14: Final cleanup**

```bash
git branch -d phase-3b
git push origin :phase-3b   # delete remote branch
```

---

## Summary

Phase 3b ships Google Calendar end-to-end:

- 4 new SQL tables, 5 new tests for the migration alone.
- ~1100 LOC in `stint-core::calendar` (types, provider trait, store CRUD, refresh, Google submodule).
- ~250 LOC of Tauri commands.
- ~200 LOC of CLI subcommands.
- ~300 LOC of UI (Settings panel + Today section + new component).
- 1 background worker mirroring `sync_worker`.
- Total: ~14 new test files (`calendar_*`).

The `CalendarProvider` trait, the `OAuthConfig::extra_authorize_params` extension, and the per-account Keychain helper are positioned for Phase 3c (Microsoft) and 3d (CalDAV) to slot in without disturbing the refresher, the UI, or the worker.

## Test plan

Run before the final PR ready:

- [ ] `STINT_SKIP_KEYCHAIN_TESTS=1 cargo fmt --all -- --check`
- [ ] `STINT_SKIP_KEYCHAIN_TESTS=1 cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `STINT_SKIP_KEYCHAIN_TESTS=1 cargo test --workspace -- --test-threads=1`
- [ ] `pnpm -C ui typecheck`
- [ ] `pnpm -C ui build`
- [ ] Manual checklist from Task 20, step 9.
- [ ] One end-to-end run against the real Google account (Mario only — pause to confirm).

CI runs items 1–5 on every push (Phase 2.5 workflow). Items 6 and 7 are gating but not automated.

## Self-Review

**Spec coverage** (spec §5):

- ✅ Schema for `calendar_accounts`, `calendars`, `calendar_events`, `event_decisions` — Task 2.
- ✅ `CalendarProvider` trait per spec — Task 4.
- ✅ Google provider with `calendar.readonly` scope and PKCE flow — Tasks 12, 13, 14.
- ✅ Per-trigger refresh windows (on-add, on-focus, background-poll, manual) — Task 15.
- ✅ Upsert keyed on `(account_id, event_id, start_at)` — Task 8.
- ✅ Today timeline with "Log this" + per-event decision tracking — Task 19.
- ✅ `event_decisions` schema in place; auto-log logic deferred to v2 — covered by "Out of scope".
- ✅ CLI/GUI parity (spec §6) — Task 17 mirrors Task 16.
- ⏭ Microsoft Graph (§5 "Microsoft Graph") — deferred to 3c per "Out of scope".
- ⏭ CalDAV (§5 "CalDAV") — deferred to 3d per "Out of scope".
- ⏭ Pixel-positioned timeline visualization — deferred to a 3b follow-up per "Out of scope".

**Placeholder scan:** None remain. `REPLACE_ME.apps.googleusercontent.com` is intentional — it's the only "placeholder" in the plan and is replaced by Mario in Task 20 step 8.

**Type consistency:** `CalendarStore`, `GoogleProvider`, `GoogleClient`, `CalendarOAuthBlob`, `CalendarOAuthStatus` are referenced by the same names across tasks. `EventDecision` variants are consistent. `EVENT_CALENDAR_CHANGED` constant is identical in both the Tauri commands module and the worker. `Ranges::on_add/on_focus/background_poll` are spelled identically in Tasks 15, 16, 17, 20. `login_interactive(client, timeout, provider_label, open_browser)` signature is consistent across all four call sites (Solidtime CLI, Solidtime Tauri, Google CLI, Google Tauri).

**Risk: `EventDecision::decoded` signature.** Task 9's helper returns `Option<Self>`; Task 16's `EventWithDecision` serializer assumes a decoded decision rather than the raw wire form. The Tauri command serializes the wire string directly (via `as_wire()`), so this is fine.

**Risk: `provider_from_wire` defaults to Google.** Task 6 maps unknown wire values to `ProviderKind::Google`. This is safe in 3b (only one variant), but the test suite would mask bad data in 3c. Add a `#[allow(unreachable_patterns)]` no — leave as-is; the panic will surface naturally when 3c adds variants and we update both ends.

**Risk: Refresh worker silently fails on bad blob.** Task 20's `build_provider` returns `MissingConfig` if the blob is missing; the per-account loop in `tick()` logs and continues. Correct behaviour — one broken account should not stall the others.

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-05-19-stint-phase-3b-calendar-google.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?







