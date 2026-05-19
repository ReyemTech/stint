# stint

Time tracker with both a CLI (`stint`) and a macOS menu-bar app (`Stint.app`)
that sync with a self-hosted Solidtime instance.

## Status

- **Phase 1** ✅ — CLI + sync + crash recovery (`phase-1-complete` tag)
- **Phase 2** ✅ — Tauri GUI + SolidJS UI + tray + dock visibility (`phase-2-complete` tag)
- **Phase 2.5** ✅ — CI baseline (lint / test / typecheck on every push and PR) (`phase-2.5-complete` tag)
- **Phase 3a** ✅ — OAuth 2.0 foundation + Solidtime OAuth sign-in (`phase-3a-complete` tag)
- **Phase 3b** — Calendar integration (Google + Microsoft + CalDAV)
- **Phase 4** — Distribution (Homebrew cask) + release CD pipeline
- **Phase 5** — Documentation site (GitHub Pages)

## Run the CLI

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

## Architecture

Both surfaces share `~/Library/Application Support/stint/stint.db`. Secrets
live in macOS Keychain under the `tech.reyem.stint.*` service prefix.

- `crates/stint-core/` — shared library: SQLite store, Solidtime client, sync
  queue, timer service, recovery
- `crates/stint-cli/` — the `stint` binary
- `crates/stint-app/` — the Tauri 2 GUI binary
- `ui/` — SolidJS + Tailwind frontend

## Sync model

Local-first. Mutations persist immediately and queue for upload. A worker
drains the queue against Solidtime with exponential backoff. Offline → work
queues up and flushes on reconnect.

## Cross-surface live updates

The GUI polls the `running_timer` table every 1s while a window is open, so
`stint start` in the terminal reflects in the menu bar popover within a
second.
