# stint

Time tracker with both a CLI (`stint`) and a macOS menu-bar app (`Stint.app`)
that sync with a self-hosted Solidtime instance.

**Docs:** [stint.reyem.tech](https://stint.reyem.tech) — install, quickstart, setup, CLI reference, troubleshooting.

## Status

- **Phase 1** ✅ — CLI + sync + crash recovery (`phase-1-complete` tag)
- **Phase 2** ✅ — Tauri GUI + SolidJS UI + tray + dock visibility (`phase-2-complete` tag)
- **Phase 2.5** ✅ — CI baseline (lint / test / typecheck on every push and PR) (`phase-2.5-complete` tag)
- **Phase 3a** ✅ — OAuth 2.0 foundation + Solidtime OAuth sign-in (`phase-3a-complete` tag)
- **Phase 3b** ✅ — Calendar integration (Google + Microsoft + CalDAV) (`phase-3b-complete` tag)
- **Phase 3c** ✅ — Solidtime down-sync (`phase-3c-complete` tag)
- **Phase 3.5** ✅ — Test coverage uplift across core / CLI / app / UI (`phase-3.5-complete` tag)
- **Phase 3d** ✅ — Post-3b UX polish + sync resilience (project picker, calendar default project, editable times, backdate start, restart-from-entry, undo logged/ignored calendar events, billable inherited from project, sync-retry storm fix, adopt-on-overlap, in-app `SyncErrorBanner` showing the conflicting Solidtime entry, `stint sync` diagnostic subcommands, workspace coverage CI job) (`phase-3d-complete` tag)
- **Phase 4** — Distribution: Homebrew cask + DMG + curl|sh installer + tauri-plugin-updater auto-update + semantic-release CD
- **Phase 5** — Documentation site (GitHub Pages)

## Install

> **Requires macOS 13 (Ventura) or later** for `Stint.app`. The CLI may run on
> macOS 12, but only macOS 13+ is officially supported. All channels are
> signed and notarized.

**Homebrew (recommended):**

    brew tap reyemtech/tap
    brew install --cask stint

**Direct DMG download:** [latest release](https://github.com/reyemtech/stint/releases/latest)

**curl | sh (CLI only):**

    curl -fsSL https://stint.reyem.tech/install.sh | sh

**curl | sh (CLI + GUI):**

    curl -fsSL https://stint.reyem.tech/install.sh | sh -s -- --gui

**Pre-release builds:**

    brew install --cask reyemtech/tap/stint-beta

Updates are delivered automatically inside the app (Settings → Updates).
You can disable auto-update or switch the update channel there.

For standalone-CLI installs (no GUI), run `stint update` to self-update.
`stint update --check` reports the available version without applying.

## Run the CLI (from source)

```bash
cargo install --path crates/stint-cli
stint config set solidtime.url https://time.reyem.ca
stint config set solidtime.token        # prompts; stored in macOS Keychain
stint config set solidtime.org <uuid>
stint config test                        # ping the API
stint start "what I'm working on"
stint stop
stint today
```

## Run the GUI (dev mode)

```bash
# one-time: install pnpm + tauri-cli
brew install pnpm
cargo install tauri-cli --version "^2.0"

cd crates/stint-app
cargo tauri dev
```

A menu-bar icon appears. Click it to toggle the popover. Use the popover's
"Open main window" button (or the tray menu's "Open Stint") for the full UI.

## Run the GUI (release build)

```bash
cargo tauri build
# produces target/release/bundle/macos/Stint.app + a .dmg
```

## Signing in with Solidtime OAuth (optional, alternative to API token)

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

### Connecting a Google Calendar (Phase 3b)

stint reads (read-only) your Google Calendar so you can convert events
into time entries with one click. The stint binary ships with a
registered Google OAuth client; you do **not** need to register your
own.

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

## Other surfaces

Beyond the menu-bar GUI and the interactive CLI, stint exposes the same
operations through three more surfaces — all driven by the same
`stint_core::verbs::*` façade, all documented at
[stint.reyem.tech](https://stint.reyem.tech):

- **Scriptable JSON** — every CLI verb accepts a global `--json` flag.
  `stint --json current`, `stint --json start "writing tests"`,
  `stint --json today | jq …`. Read verbs emit the canonical view
  type; admin verbs emit a structured acknowledgement.
- **AI integration (MCP)** — `stint mcp` is an MCP server over stdio
  exposing 8 tools. `stint skill install <claude|codex|opencode>`
  wires it into your harness in one shot, including a bundled
  `SKILL.md` that teaches the agent when and how to call each tool.
- **Loopback HTTP API** — off by default; enable with
  `stint config set api.enabled true`. The GUI binds `127.0.0.1` on
  an ephemeral port; discover the URL via `stint api info`. Endpoints
  live under `/v1/`.
- **`stint://` URL scheme** — for Raycast / Alfred / Shortcuts. Supports
  `stint://start?description=…&project=…&billable=true`,
  `stint://stop`, `stint://current`, `stint://entry/<local-uuid>`.

The `stint(1)` man page (`man stint`) ships inside the Homebrew cask;
for `cargo install` / `curl|sh` users, `scripts/install-man.sh`
installs it to `/usr/local/share/man/man1/`.

## Architecture

Both surfaces share `~/Library/Application Support/stint/stint.db`. Secrets
live in macOS Keychain under the `tech.reyem.stint.*` service prefix.

- `crates/stint-core/` — shared library: SQLite store, Solidtime client, sync
  queue, timer service, recovery, MCP/HTTP `verbs::` façade
- `crates/stint-cli/` — the `stint` binary
- `crates/stint-app/` — the Tauri 2 GUI binary (hosts the loopback HTTP API)
- `ui/` — SolidJS + Tailwind frontend

## Sync model

Local-first. Mutations persist immediately and queue for upload. A worker
drains the queue against Solidtime with exponential backoff. Offline → work
queues up and flushes on reconnect.

## Cross-surface live updates

The GUI polls the `running_timer` table every 1s while a window is open, so
`stint start` in the terminal reflects in the menu bar popover within a
second.
