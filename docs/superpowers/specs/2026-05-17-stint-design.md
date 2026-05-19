# stint — Design Spec

A macOS time tracker with both a CLI and a Tauri-based GUI that syncs with a self-hosted Solidtime instance.

- **Status:** Design approved 2026-05-17
- **Target platform:** macOS (Apple Silicon + Intel)
- **Source language:** Rust (core, CLI, Tauri shell) + TypeScript / SolidJS (UI)

## 1. What we're building

`stint` is a personal time-tracking application for one user with a self-hosted Solidtime instance at `https://time.reyem.ca`. It provides three surfaces over a shared core:

1. A macOS menu-bar popover (always-accessible quick start/stop, current timer, today's totals)
2. A macOS main window (history browsing, editing, summaries, calendar view, settings)
3. A command-line interface (`stint start "..."`, `stint stop`, `stint today`, `stint config ...`, etc.)

All three are backed by a single SQLite database and a single Rust core library. Either UI surface can be used independently; both reflect each other's changes within a few seconds.

The system supports working offline: local mutations are persisted and queued, then drained to Solidtime when the network is available. Calendar events from Google, Microsoft 365, and any CalDAV server can be viewed alongside time entries and logged into Solidtime with a single click. Calendar data is prepared to support future automatic logging without a schema migration.

### Primary use cases

1. Click the menu bar icon, type a description, click start. Counter ticks. Click stop. Entry syncs to Solidtime.
2. From terminal: `stint start "fixing the export bug" --project Tet`. The menu bar popover reflects the running timer within ~5 seconds.
3. Open main window → review the week → edit a past entry → save. Edit syncs to Solidtime.
4. Disconnect from the network. Start and stop several timers. Reconnect. Queued entries flush automatically.
5. App crashes while a timer is running. Relaunch. Timer is restored at last heartbeat. User chooses to keep, stop, or discard.
6. Open main window. See today's Google + iCloud calendar events on a timeline. Click "Log this" next to a meeting → a pre-filled time entry is created, ready for project assignment.

## 2. Architecture

### Cargo workspace

```
stint/
├─ Cargo.toml                      # workspace
├─ crates/
│  ├─ stint-core/                 # shared library — the only place business logic lives
│  │   ├─ src/lib.rs
│  │   ├─ src/timer.rs            # timer state machine
│  │   ├─ src/store/              # SQLite, schema, migrations
│  │   ├─ src/solidtime/          # Solidtime API client + DTOs
│  │   ├─ src/calendar/           # provider trait + Google/Microsoft/CalDAV impls
│  │   ├─ src/sync/               # offline queue + reconciliation worker
│  │   ├─ src/config/             # settings + Keychain bridge
│  │   └─ src/error.rs            # thiserror-based error type
│  ├─ stint-cli/                  # `stint` binary (clap)
│  │   └─ src/main.rs
│  └─ stint-app/                  # `stint-app` Tauri binary
│      ├─ src/main.rs              # tray, window mgmt, tauri commands
│      └─ src/commands.rs          # thin #[tauri::command] wrappers over stint-core
└─ ui/                              # SolidJS frontend (Tauri convention)
   ├─ src/...
   ├─ package.json
   └─ vite.config.ts
```

**Core principle.** All business logic lives in `stint-core`. CLI and GUI binaries are thin: they translate user input → core calls → output. One bug fix in core fixes both surfaces; one test suite covers the logic; the core can be driven test-first before either UI exists.

### CLI and GUI coexistence (Architecture A)

Both binaries depend on `stint-core` and open the same SQLite database file. There is no daemon. The GUI, when running, hosts a long-lived sync worker; the CLI runs a brief drain pass per invocation before exiting. When both run concurrently, the GUI polls SQLite (cheap on a single-user DB) to reflect CLI-driven changes within a few seconds. A future improvement may add a notify mechanism (e.g., file-watch or SIGUSR1) to remove polling latency.

### Rust dependencies

| Crate | Purpose |
|---|---|
| `tokio` | async runtime |
| `reqwest` | HTTP client (Solidtime, Google, Microsoft, CalDAV) |
| `sqlx` | SQLite + compile-time checked queries + migrations |
| `clap` v4 | CLI parsing (derive API) |
| `tauri` v2 | GUI shell, tray, menu bar, system integration |
| `keyring` | macOS Keychain access |
| `serde`, `serde_json` | serialization |
| `thiserror` (core) / `anyhow` (binaries) | error handling |
| `tracing` + `tracing-subscriber` | structured logging |
| `chrono` | time/date |
| `oauth2` | OAuth 2.0 PKCE flows (Google, Microsoft) |
| `icalendar` | iCal parsing for CalDAV |
| `quick-xml` | CalDAV PROPFIND/REPORT XML |
| `uuid` | local UUIDs |
| `tray-icon` (via Tauri v2) | menu bar icon |
| `wiremock` (dev) | HTTP mocks for tests |
| `assert_cmd` (dev) | CLI integration tests |

### Frontend (Tauri)

- **SolidJS** with Tailwind CSS. Chosen for fine-grained reactivity (the ticking timer updates a single DOM node per second without re-rendering), small bundle (~7 KB), and forward-compatible mental model (signals are now standard in Vue, Angular, Preact, Svelte 5, and a proposed JavaScript primitive).
- The popover UI is a SolidJS app; the main window is the same SolidJS app rendered at a different route.

## 3. Data model

SQLite database lives at `~/Library/Application Support/stint/stint.db`. Migrations are managed by `sqlx::migrate!`. All timestamps are RFC 3339 UTC.

### Core tables

```sql
-- Non-secret config. Tokens go to Keychain, not here.
CREATE TABLE settings (
  key         TEXT PRIMARY KEY,
  value       TEXT NOT NULL,
  updated_at  TEXT NOT NULL
);

CREATE TABLE time_entries (
  local_uuid      TEXT PRIMARY KEY,            -- always present, generated on create
  solidtime_id    TEXT UNIQUE,                 -- NULL until first successful push
  description     TEXT NOT NULL DEFAULT '',
  project_id      TEXT,                        -- solidtime project uuid
  task_id         TEXT,
  start_at        TEXT NOT NULL,
  end_at          TEXT,                        -- NULL while running
  billable        INTEGER NOT NULL DEFAULT 0,
  source          TEXT NOT NULL,               -- 'cli' | 'gui' | 'calendar' | 'imported'
  source_event_id TEXT,                        -- future: link to calendar_events.id
  sync_state      TEXT NOT NULL,               -- 'synced' | 'dirty' | 'pending_create' | 'pending_delete'
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL
);
CREATE INDEX idx_time_entries_start ON time_entries(start_at);
CREATE INDEX idx_time_entries_sync  ON time_entries(sync_state) WHERE sync_state != 'synced';

CREATE TABLE entry_tags (
  local_uuid  TEXT NOT NULL REFERENCES time_entries(local_uuid) ON DELETE CASCADE,
  tag_id      TEXT NOT NULL,
  PRIMARY KEY (local_uuid, tag_id)
);

-- At most one row. Survives crash. Heartbeat updated every ~5 s while a timer is active.
CREATE TABLE running_timer (
  id            INTEGER PRIMARY KEY CHECK (id = 1),
  local_uuid    TEXT NOT NULL REFERENCES time_entries(local_uuid),
  heartbeat_at  TEXT NOT NULL
);
```

### Reference data (read-only mirror)

```sql
CREATE TABLE projects (
  id          TEXT PRIMARY KEY,                -- solidtime uuid
  name        TEXT NOT NULL,
  color       TEXT,
  client_id   TEXT,
  archived    INTEGER NOT NULL DEFAULT 0,
  fetched_at  TEXT NOT NULL
);

CREATE TABLE tasks (
  id          TEXT PRIMARY KEY,
  project_id  TEXT NOT NULL REFERENCES projects(id),
  name        TEXT NOT NULL,
  done        INTEGER NOT NULL DEFAULT 0,
  fetched_at  TEXT NOT NULL
);

CREATE TABLE tags (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  fetched_at  TEXT NOT NULL
);
```

### Sync queue

```sql
CREATE TABLE sync_queue (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  op            TEXT NOT NULL,                 -- 'create_entry' | 'update_entry' | 'delete_entry'
  payload       TEXT NOT NULL,                 -- JSON body
  attempts      INTEGER NOT NULL DEFAULT 0,
  last_error    TEXT,
  enqueued_at   TEXT NOT NULL,
  next_try_at   TEXT NOT NULL                  -- exponential backoff target
);
```

### Calendar

```sql
CREATE TABLE calendar_accounts (
  id            TEXT PRIMARY KEY,              -- local uuid
  provider      TEXT NOT NULL,                 -- 'google' | 'microsoft' | 'caldav'
  display_name  TEXT NOT NULL,
  identifier    TEXT NOT NULL,                 -- email for OAuth, URL for CalDAV
  caldav_url    TEXT,                          -- nullable
  enabled       INTEGER NOT NULL DEFAULT 1,
  created_at    TEXT NOT NULL
);

CREATE TABLE calendars (
  id          TEXT PRIMARY KEY,
  account_id  TEXT NOT NULL REFERENCES calendar_accounts(id) ON DELETE CASCADE,
  name        TEXT NOT NULL,
  color       TEXT,
  included    INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE calendar_events (
  id              TEXT NOT NULL,
  account_id      TEXT NOT NULL REFERENCES calendar_accounts(id) ON DELETE CASCADE,
  calendar_id     TEXT NOT NULL REFERENCES calendars(id) ON DELETE CASCADE,
  title           TEXT NOT NULL,
  start_at        TEXT NOT NULL,
  end_at          TEXT NOT NULL,
  is_all_day      INTEGER NOT NULL DEFAULT 0,
  attendee_status TEXT,                        -- 'accepted' | 'declined' | 'tentative' | NULL
  recurring_root  TEXT,                        -- root event id if this is an expanded instance
  fetched_at      TEXT NOT NULL,
  PRIMARY KEY (account_id, id, start_at)
);

-- Prep for future auto-log; tracks per-event decisions
CREATE TABLE event_decisions (
  account_id        TEXT NOT NULL,
  event_id          TEXT NOT NULL,
  event_start       TEXT NOT NULL,
  decision          TEXT NOT NULL,             -- 'ignored' | 'logged_manual' | 'logged_auto'
  linked_local_uuid TEXT REFERENCES time_entries(local_uuid),
  decided_at        TEXT NOT NULL,
  PRIMARY KEY (account_id, event_id, event_start)
);
```

### Identity rules

- `local_uuid` is the stable identity from the moment a row is created locally, including before it has ever reached Solidtime. This is the engine of offline-first.
- `solidtime_id` is populated on first successful sync.
- The `running_timer.CHECK (id = 1)` enforces at most one active timer.
- `(account_id, id, start_at)` is the calendar-event key so recurring instances coexist with their parent.

## 4. Sync model

### Mutation pipeline

Every state-changing operation flows through `stint-core::Store::mutate(op)`:

```
User action (CLI or GUI)
    ↓
[SQLite transaction]
  1. Apply change locally → time_entries.sync_state ∈ {pending_create, dirty, pending_delete}
  2. Append matching record to sync_queue
  3. Commit
    ↓
Notify sync worker (in-process channel)
    ↓
Sync worker drains sync_queue (one op at a time per entry):
  - Success → time_entries.sync_state = 'synced', set solidtime_id, delete sync_queue row
  - 4xx → mark row with error, surface to user; do not retry blindly
  - 5xx / network error → exponential backoff (next_try_at += 2^attempts s, capped ~5 min)
```

The local mutation and the queue write share a transaction; either both happen or neither does. The user always sees an instant local update.

### Pull refresher

Reference data and calendar events are pulled on a schedule:

- On launch (once)
- On main-window focus (throttled to once per 5 min)
- On manual refresh
- Background poll every 15 min while the GUI is running (the CLI does not poll; it refreshes opportunistically when a command needs current data)

Pulls are full upserts with a `fetched_at` stamp. v1 does not implement delta sync; reference lists are small.

### Conflict resolution

v1 uses **client-wins on push**. A local edit overwrites server-side state. This is acceptable for a single-user, mostly single-device tool; concurrent edits are rare. Anything more sophisticated (CRDT, version vectors) is deferred.

Explicit error handling:

- **422 (stale reference)** — referenced project/task was deleted server-side. Mark sync_queue row failed, surface to user.
- **401 (auth)** — sync pauses; GUI shows a "reconfigure token" prompt.

The server is the source of truth for IDs and canonical timestamps; we always write back the values it returns.

### Crash recovery for the running timer

On startup the running-timer table is consulted:

1. No row → nothing to do.
2. `heartbeat_at < 60 s` old → another stint process is live; attach to it.
3. `60 s ≤ age ≤ 10 min` → recent crash. Silently resume; start a new heartbeat.
4. `age > 10 min` → ambiguous. Prompt user: "stint stopped at 14:32 with timer still running ('Code review', 1h22m elapsed). Keep running, stop at last heartbeat, or discard?"

The heartbeat is a single-row update every 5 s while a timer is active.

### Why no daemon

A separate sync daemon was considered and rejected. The CLI invocations are brief but enqueue mutations regardless; the GUI is the primary long-running surface. A daemon adds launchd plumbing, IPC, and lifecycle complexity for marginal benefit. The accepted trade-off: if neither the GUI nor a CLI command runs for hours, the queue does not drain until next interaction, at which point it catches up in seconds.

## 5. Calendar integration (and Solidtime OAuth)

The calendar phase builds OAuth 2.0 PKCE infrastructure (authorize → redirect → exchange → store + refresh tokens in Keychain). The same machinery is reused to offer **Solidtime OAuth** alongside the existing API-token path:

- The user can pick **API token** (paste a personal access token, as today) or **Sign in with Solidtime** (browser-based OAuth flow) in Settings.
- API-token connections keep working unchanged; the OAuth path adds a `SolidtimeAuth` enum so the rest of `stint-core` doesn't care which auth shape was used.
- Refresh tokens land in Keychain under `tech.reyem.stint.solidtime.oauth.*`, separate from the existing `tech.reyem.stint.solidtime` token entry.

The four OAuth providers (Google, Microsoft, CalDAV when applicable, and Solidtime) share:

- A common `OAuthClient` wrapper around the `oauth2` crate
- A common redirect-capture HTTP server bound to `127.0.0.1:<random>`
- A common refresh loop that writes refreshed tokens back to Keychain

### The trait

```rust
#[async_trait]
pub trait CalendarProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    async fn list_calendars(&self) -> Result<Vec<RemoteCalendar>>;
    async fn list_events(&self, calendar_id: &str, range: TimeRange) -> Result<Vec<CalendarEvent>>;
}
```

A `CalendarAccount` row produces a concrete `Box<dyn CalendarProvider>` at runtime; auth material is fetched from Keychain on construction.

### Google Calendar

- OAuth 2.0 with PKCE
- Scope: `https://www.googleapis.com/auth/calendar.readonly`
- Flow: open system browser → user grants → redirect to `http://127.0.0.1:<random>/callback` → stint's one-shot local server captures the code → exchange for access + refresh tokens → store in Keychain
- Token refresh is automatic; refreshed tokens are written back to Keychain

### Microsoft Graph

- OAuth 2.0 with PKCE; same flow shape as Google
- Authorize: `https://login.microsoftonline.com/common/oauth2/v2.0/authorize`
- Scope: `Calendars.Read offline_access`
- API base: `https://graph.microsoft.com/v1.0/me/calendars`
- Requires one-time Azure app registration; client ID baked into stint, no client secret (PKCE)

### CalDAV

Authentication variants:

```rust
pub enum CalDavAuth {
    Basic       { username: String, password_keychain_ref: String },
    AppPassword { username: String, app_password_keychain_ref: String }, // Fastmail, iCloud
    OAuth       { /* rare; e.g. Google CalDAV — prefer Google native */ },
}
```

Discovery:

1. PROPFIND on the principal URL → `calendar-home-set`
2. PROPFIND on the calendar-home-set URL → list of calendars
3. REPORT with `calendar-query` filter for date ranges → multiple `.ics` blobs

The `icalendar` crate parses VEVENT, RRULE expansion, EXDATE, and modified instances. All-day events are stored with `is_all_day = 1` and time stripped.

### Event refresh strategy

| Trigger | Range |
|---|---|
| Account added | last 7 days + next 14 days |
| Launch / window focus | next 7 days |
| Background poll (every 15 min while GUI runs) | last 1 day + next 7 days |
| Manual refresh | full window |

Upserts keyed on `(account_id, event_id, start_at)`.

### UI placement

- **Main window — Today / Week view:** events render alongside time entries on a timeline. Time entries are solid blocks; calendar events are outlined blocks colored by source calendar with a provider icon. Each event has a "Log this" action that pre-fills a new entry (title, start, end, `source = 'calendar'`, `source_event_id`). Logged events show a checkmark; ignored events can be dismissed (writes to `event_decisions`).
- **Menu-bar popover:** does not show calendar events; stays focused on the current timer.

### Auto-log preparation

Schema and decision tracking are in place. v2 will add: an `auto_log_rules` table (filters on calendar, attendee count, keyword), a worker that creates entries when rules match incoming events, and a UI for managing rules. v1 ships no rules engine.

## 6. Configuration and secrets

**Non-secret config** is stored in the `settings` table. Examples: Solidtime base URL, organization UUID, default project UUID, UI theme.

**Secrets** are stored in macOS Keychain via `keyring`, under the service prefix `tech.reyem.stint`:

- `tech.reyem.stint.solidtime` → Solidtime API token
- `tech.reyem.stint.calendar.<account_uuid>` → OAuth tokens or CalDAV password

### CLI surface (parity with GUI)

Every setting reachable from the GUI is also reachable from the CLI:

```
stint config set solidtime.url https://time.reyem.ca
stint config set solidtime.token                    # interactive prompt; never echoed
stint config set solidtime.org <uuid>               # autocompleted from API once token is set
stint config set solidtime.default-project <uuid|name>
stint config show                                   # masks tokens as ••••
stint config test                                   # GET /api/v1/users/me → ✓ or error

stint calendar add google
stint calendar add microsoft
stint calendar add caldav --url <url> --user <user>
stint calendar list
stint calendar test <account-id>
stint calendar remove <account-id>
stint calendar calendars <account-id>                # toggle inclusion per calendar
```

### First-run behaviour

If config is empty: CLI exits with a helpful message describing the missing values; GUI opens the Settings panel.

## 7. Testing strategy

**Layer 1 — `stint-core` unit tests** (the bulk of coverage):

- Timer state machine (start → tick → stop → edit → delete)
- Sync queue draining (HTTP mocked via `wiremock`)
- Conflict / retry behavior
- iCal RRULE expansion against known fixtures
- SQL migration tests

**Layer 2 — `stint-cli` integration tests** (`assert_cmd` driving the binary against a temp DB + mock Solidtime):

- `stint start "foo"` writes a row and queues sync
- `stint stop` sets `end_at`
- `stint config test` returns expected status codes against the mock

**Layer 3 — Tauri command smoke tests** (minimal): verify that `#[tauri::command]` wrappers correctly call into `stint-core`.

**Not in CI:** the SolidJS UI (manual testing in v1), real Solidtime calls, real OAuth flows.

The full Rust suite runs in seconds.

## 8. Distribution (Homebrew)

stint ships as a single Homebrew cask. The CLI binary is bundled inside `Stint.app` and the cask symlinks it out to the user's `$PATH`, so one install yields both surfaces.

```
brew tap reyemtech/tap     # one-time setup
brew install stint
```

After the one-time tap, day-to-day commands use the short name: `brew upgrade stint`, `brew uninstall stint`, etc. The fully-qualified form (`reyemtech/tap/stint`) is only required if another tap installs a colliding `stint`, which is not expected.

After install:

- `/Applications/Stint.app` — clickable GUI, menu bar icon
- `/opt/homebrew/bin/stint` (Apple Silicon) or `/usr/local/bin/stint` (Intel) — symlinked CLI

This is the same pattern 1Password uses to ship the `op` CLI alongside its app.

### Tap layout

A separate repository, `github.com/reyemtech/homebrew-tap`:

```
homebrew-tap/
└─ Casks/
   └─ stint.rb
```

No separate formula is needed; the cask handles both artifacts.

### .app bundle structure

The Tauri build is configured to place both binaries inside the bundle:

```
Stint.app/
└─ Contents/
   ├─ Info.plist
   ├─ MacOS/
   │   ├─ Stint              # GUI entry point (`stint-app` binary, renamed)
   │   └─ stint              # CLI binary (`stint-cli` binary)
   └─ Resources/
```

The CLI is built alongside the GUI in the release pipeline and copied into `Contents/MacOS/` before signing.

### Cask formula

```ruby
cask "stint" do
  version "0.1.0"
  sha256 "<dmg sha256>"

  url "https://github.com/reyemtech/stint/releases/download/v#{version}/Stint-#{version}.dmg"
  name "Stint"
  desc "Time tracker that syncs with Solidtime (CLI + menu bar app)"
  homepage "https://github.com/reyemtech/stint"

  app "Stint.app"
  binary "#{appdir}/Stint.app/Contents/MacOS/stint"

  zap trash: [
    "~/Library/Application Support/stint",
    "~/Library/Preferences/tech.reyem.stint.plist",
    "~/Library/Caches/tech.reyem.stint",
  ]
end
```

The `binary` stanza creates the symlink for CLI access. The `zap` stanza ensures `brew uninstall --zap stint` removes the SQLite database and preferences.

### Code signing and notarization

The cask requires a signed and notarized `.app` and `.dmg` for clean install — otherwise macOS Gatekeeper blocks first-launch. This requires an Apple Developer Program membership.

- **Developer ID Application certificate** — signs the `.app` bundle and the `.dmg`
- **Notarization** — uploads to Apple, gets a stapled ticket, no Gatekeeper warning

A fallback path (unsigned, with a README note instructing users to right-click → Open on first launch) is acceptable only if no Apple Developer account is available. v1 should target signed + notarized.

### Release pipeline (GitHub Actions)

On a tag push (`vX.Y.Z`):

1. Build the CLI (`stint-cli`) as a universal binary (`x86_64-apple-darwin` + `aarch64-apple-darwin` → `lipo`).
2. Build the Tauri app (`stint-app`) for both architectures, then bundle a universal `.app`.
3. Copy the CLI binary into `Stint.app/Contents/MacOS/stint`.
4. Sign and notarize the `.app` (signature covers the embedded CLI). Build the `.dmg` containing the signed app and sign + notarize the `.dmg`.
5. Compute SHA256 of the `.dmg`.
6. Create a GitHub Release with the `.dmg` attached.
7. Open a PR (or push directly) to `homebrew-tap` updating the cask `version` and `sha256`.

The tap repo is intentionally separate from the application repo so the cask can be updated without churning the main code history.

### Auto-update

v1 ships no in-app update mechanism. Users update via `brew upgrade stint`, which updates both the GUI and the bundled CLI in one step. A future in-app updater (e.g., `tauri-plugin-updater`) is deferred.

## 9. Continuous integration (Phase 2.5)

A small, focused CI baseline lands as Phase 2.5 — separate from the release pipeline — so every push and pull request gets fast feedback before more code accumulates.

**GitHub Actions workflow** (`.github/workflows/ci.yml`), macOS runners only since the app targets macOS:

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace -- --test-threads=1`
4. `pnpm install --frozen-lockfile`
5. `pnpm -C ui typecheck`
6. `pnpm -C ui build`

The workflow caches Cargo (`~/.cargo/registry`, `target/`) and pnpm (`~/.local/share/pnpm/store`) to keep runs under a few minutes after warm-up. Required check on PRs merging to `main`.

## 10. Release pipeline (CD, part of Phase 4)

Triggered by tag push `vX.Y.Z`. Runs on a macOS runner with the signing identity injected via GitHub secrets:

1. Build both architectures (`x86_64-apple-darwin` + `aarch64-apple-darwin`) for the CLI and Tauri app.
2. `lipo` them into universal binaries.
3. Copy the CLI into `Stint.app/Contents/MacOS/stint` (alongside the GUI binary).
4. Sign the `.app` with the Developer ID certificate (`signing_identity` from secrets).
5. Submit to Apple notarization, wait for the ticket, staple it.
6. Package the signed app into a `.dmg`, sign and notarize the `.dmg` too.
7. Compute the SHA256 of the `.dmg`.
8. Create a GitHub Release at the tag with the `.dmg` attached.
9. Open a PR (or commit directly) to `reyemtech/homebrew-tap` updating the cask's `version` and `sha256`.

Secrets required: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD` (app-specific password), `APPLE_TEAM_ID`, plus a `HOMEBREW_TAP_TOKEN` (fine-grained PAT scoped to the tap repo).

## 11. Documentation site (Phase 5)

Published to **GitHub Pages** from `docs/` via a separate workflow that triggers on pushes to `main`. Tooling: **VitePress** (lightest fit; we already have a JS ecosystem in `ui/`) or **Astro Starlight** (better default look + search, slightly larger). Pick during Phase 5 planning.

Contents:

- **Getting started** — install via brew, configure Solidtime URL + token (or OAuth), first timer
- **CLI reference** — every command with arguments and examples (auto-generated from `clap` where practical)
- **GUI tour** — popover, main window, menu bar shortcuts, with screenshots
- **Architecture** — workspace layout, data model, sync model, recovery, cross-surface coordination, the OAuth providers
- **Contributing** — repo layout, how to run dev, where to add tests, the testing layers, commit conventions
- **FAQ + troubleshooting** — common Keychain prompts, sync failures, log paths

The site URL: `https://reyemtech.github.io/stint/` (default) or a custom domain like `stint.reyem.tech` once stable.

## 12. Out of scope

To prevent scope creep, the following are explicitly excluded from the current roadmap:

- **Auto-log from calendar events** — schema prepped, logic deferred to v2.
- **Writing to calendars** — read-only in v1.
- **Multi-organization Solidtime** — one configured org at a time.
- **Reports, invoicing, exports** — daily and weekly summary views only; richer reporting lives in Solidtime's web UI.
- **Idle detection** — needs accessibility permissions and behavior design; deferred.
- **Cross-platform support** — macOS only. The CLI may work on Linux/Windows but is not a target.
- **Plugins or extensibility surfaces.**
- **Real-time push from Solidtime** — Solidtime does not expose a WebSocket; sync is pull-based on a timer.
- **Sophisticated conflict-resolution UI** — client-wins on push; unrecoverable errors are surfaced.

## 13. Decision summary

| Decision | Value |
|---|---|
| Name | **stint** |
| Platform | macOS |
| Languages | Rust + TypeScript (SolidJS) |
| Architecture | Cargo workspace: `stint-core` shared lib + `stint-cli` + `stint-app` (Tauri) |
| Storage | SQLite at `~/Library/Application Support/stint/stint.db` |
| Secrets | macOS Keychain (`tech.reyem.stint.*`) |
| UI surfaces | Menu-bar popover + main window + CLI |
| Sync | Local-first, mutation queue, exponential backoff, client-wins |
| Calendar | Read-only Google + Microsoft + CalDAV with "Log this" UX |
| Auto-log | Schema prepared; logic deferred to v2 |
| Frontend | SolidJS + Tailwind |
| Distribution | Single Homebrew cask `reyemtech/tap/stint` (bundles CLI inside `Stint.app`) |
| Signing | Apple Developer ID + notarization for the `.app` and `.dmg` |
| Solidtime auth | API token (today) **and** OAuth 2.0 PKCE (Phase 3) |
| CI baseline | Phase 2.5 — fmt/clippy/test/typecheck/build on every push and PR |
| Release pipeline | Phase 4 — tag-triggered sign + notarize + publish to brew tap |
| Docs site | Phase 5 — VitePress (TBD Starlight) on GitHub Pages |

## Phase roadmap

| Phase | Scope | Status |
|---|---|---|
| 1 | CLI + sync + crash recovery | ✅ shipped (`phase-1-complete`) |
| 2 | Tauri GUI + SolidJS UI | ✅ shipped (`phase-2-complete`) |
| 2.5 | CI baseline (lint / test / typecheck on PR) | planned |
| 3 | Calendar (Google + MS + CalDAV) + Solidtime OAuth | planned |
| 4 | Distribution + release CD pipeline | planned |
| 5 | Documentation site (GitHub Pages) | planned |
