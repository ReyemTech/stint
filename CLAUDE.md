# CLAUDE.md — Repo notes for AI coding agents

This file is the entry point for any AI coding assistant (Claude Code,
Cursor, Codex, OpenCode, etc.) working in this repository. Read it before
touching code. Per-harness skills + MCP integration are managed via
`stint skill install <harness>` — see "Gotchas" below.

## What stint is

A macOS time tracker that syncs with a self-hosted [Solidtime](https://www.solidtime.io)
instance. Two surfaces over a shared Rust core:

- **`stint`** — CLI binary
- **`Stint.app`** — Tauri 2 + SolidJS menu-bar + main-window app

Both open the same SQLite database at
`~/Library/Application Support/stint/stint.db`. Secrets live in macOS Keychain
under the `tech.reyem.stint.*` service prefix. Sync is local-first: every
mutation persists immediately and queues for upload to Solidtime; a worker
drains with exponential backoff.

## Repository layout

```
Cargo.toml                            # workspace
pnpm-workspace.yaml                   # JS workspace (ui/)
rust-toolchain.toml                   # pinned Rust version
README.md                             # user-facing intro + status

crates/
  stint-core/                         # all business logic — store, sync,
                                      # timer, Solidtime client, recovery
  stint-cli/                          # `stint` binary — thin clap wrappers
                                      # over stint-core
  stint-app/                          # Tauri 2 binary — windows, tray,
                                      # menu bar, IPC commands

ui/                                   # SolidJS + Tailwind frontend
  src/
    components/                       # generic + ui/ primitives (Button,
                                      # Toggle, StatusDot, Pill, …)
    routes/                           # Popover, Today, Settings, About
    stores/                           # createSignal-based stores (timer)
    lib/                              # helpers (openSolidtime, useHotkey)

docs/superpowers/
  specs/2026-05-17-stint-design.md    # source of truth for design
  plans/*.md                          # one plan per phase
```

The `stint-core` crate is the only place business logic lives. CLI and Tauri
commands MUST stay thin: parse input → call stint-core → format output.

## How to build / run / test

```bash
# Build everything
cargo build --workspace

# Run the Rust test suite (single-threaded — tests share Keychain)
cargo test --workspace -- --test-threads=1

# Lint
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

# CLI dev loop
cargo run -p stint-cli -- start "writing tests"
cargo run -p stint-cli -- stop

# GUI dev loop (Tauri auto-reloads Rust on change; Vite HMR for the UI)
cd crates/stint-app
cargo tauri dev

# UI-only loop (Vite without Tauri shell — limited; most things need IPC)
cd ui
pnpm dev

# UI typecheck + production build
cd ui
pnpm typecheck
pnpm build
```

First-time setup on a fresh machine:

```bash
brew install pnpm rust          # rust via brew is OK (Homebrew skips rustup)
cargo install tauri-cli --version "^2.0" --locked
pnpm install                    # at repo root
```

## Conventions

### Commits

Conventional Commits, scoped where useful:

```
feat(app): tray icon with popover toggle
fix(sync): include member_id in time-entry requests
chore(ui): scaffold SolidJS + Vite + Tailwind 4
test(core): cover sync_queue backoff
docs: roadmap update
refactor(ui): extract shared primitives
```

Keep messages tight. Body explains the *why* if the diff doesn't already.

### Branching

- `main` — current shipped state
- `phase-N` — feature branches; merged into `main` via a merge commit once tagged
- `phase-N-complete` — annotated tag marking each phase's release

For new phases, branch from `main`:

```bash
git checkout -b phase-2.5
```

### Testing discipline

- **TDD for `stint-core`** — write the failing test, then the implementation.
  See any of `crates/stint-core/tests/*.rs` for the pattern. New verbs land
  with their happy path **and** at least one error / edge case test.
- **Integration over unit** for store-level code — tests run against a real
  SQLite tempdir via `tests/common/mod.rs` `setup()`. Helpers like
  `seed_projects` / `seed_tasks` use the public `Reference` API rather than
  raw SQL so the tests don't drift when the schema evolves.
- **Wiremock for HTTP** — `crates/stint-core/tests/solidtime.rs` and
  `sync_*` files show the pattern. Real network calls are forbidden in tests.
- **`assert_cmd` + `insta` for CLI** — `crates/stint-cli/tests/cli_e2e.rs`
  for end-to-end exercise of the binary; `tests/verbs_json.rs` golden
  snapshots lock the `--json` output shape per verb.
- **`tower::ServiceExt::oneshot` for HTTP API** — see
  `crates/stint-app/tests/http_api.rs`. In-process exercise of the same
  `axum::Router` production uses; never binds a real socket.
- **MCP server: spawn-and-talk** — `crates/stint-cli/tests/mcp_e2e.rs`
  spawns `stint mcp` as a child process and exchanges line-delimited
  JSON-RPC over stdio. Use this pattern for any new tool.
- **Skill installer: tempdir HOME** — `crates/stint-cli/tests/skill_*.rs`
  swap `HOME` to a tempdir before exercising file mutations; never touches
  the user's real `~/.claude`, `~/.codex`, or `~/.config/opencode`.
- **UI**: `vitest` + `jsdom` + `@solidjs/testing-library`. Tests live next
  to whatever they cover (`ui/src/test/{components,routes,stores,lib}/`).
  Run via `pnpm test` (watch) or `pnpm test:coverage` (one-shot report).
  Manual smoke (`scripts/dev-app.sh` click-through) is still required for
  visual / UX changes — compile-pass and unit-pass don't prove the UI works.
- **`vi.mock` factories are hoisted** — top-level `const`s referenced by a
  factory must be wrapped in `vi.hoisted(() => …)` or the factory runs
  before they're initialized. See `ui/src/test/stores/timer.test.ts`.

### Coverage standards

- **One command, all surfaces**: `scripts/coverage.sh` runs Rust
  (`cargo-llvm-cov` for `stint-core` / `stint-cli` / `stint-app`) AND UI
  (`vitest --coverage`), then prints a unified per-surface table:

  ```
    surface       lines (covered/total)   functions  status
    -------       ---------------------   ---------  ------
    stint-core     94.0%  ( 3023/ 3217)    92.4%     ✅
    stint-cli      82.1%  ( 1234/ 1504)    81.2%     ✅
    stint-app      83.7%  (  947/ 1131)    73.0%     ✅
    ui             88.8%  ( 1879/ 2116)    89.0%     ✅
    TOTAL          86.4%  ( 7083/ 7968)    83.1%
  ```

  Exits non-zero if any surface drops below `COVERAGE_FLOOR` (default 80%).
  CI consumes the same script; the local report matches CI.
- **Threshold — 80% lines per surface, enforced.** Below 80% on any of
  `stint-core`, `stint-cli`, `stint-app`, or `ui` fails the script. New
  code lands with tests sufficient to keep its own file at or above the
  surface average.
- **Use `SKIP_RUST=1` or `SKIP_UI=1`** to skip half the run when iterating
  on tests for just one side. The unified table still prints from the most
  recent reports on disk (`target/coverage/lcov.info` +
  `ui/coverage/coverage-summary.json`).
- **What's NOT counted toward coverage** (excluded in
  `scripts/coverage.sh::IGNORE_RE` and `ui/vitest.config.ts::exclude`):
  - `tests/` directories themselves
  - `stint-app/src/{main,menu,tray,windows,logging,app_state,*_worker,updater}.rs`
    and `commands/ui.rs` — Tauri runtime wiring (system menu, dock, async
    workers, updater plugin); exercises native macOS APIs the test
    harness can't drive
  - `stint-cli/src/cmd/{mcp,calendar,config_login,update}.rs`,
    `mcp/mod.rs`, `skill/picker.rs` — subprocess / interactive / OAuth /
    signed-release surfaces. The `stint mcp` subcommand is exercised
    indirectly by `tests/mcp_e2e.rs` (subprocess) AND directly by the
    `mcp/tools.rs` `#[cfg(test)] mod tests` block.
  - `ui/src/main.tsx`, `*.d.ts`, `src/test/**`
- **Coverage gates that DON'T move the goalposts**: golden snapshots
  (`crates/stint-cli/tests/verbs_json.rs`), MCP e2e
  (`crates/stint-cli/tests/mcp_e2e.rs`), and HTTP integration
  (`crates/stint-app/tests/http_api.rs`) each lock one wire shape —
  they don't trade off against per-function coverage. Aim for both.
- **Tests must pass before coverage is meaningful** — don't paper over a
  failing suite. CI runs `cargo test --workspace -- --test-threads=1` and
  `pnpm test:run` as separate gates before the coverage job.

### Code style

- Rust: idiomatic, no unwrap in production paths (only in tests), errors
  flow through `stint_core::Error` (typed enum) at the library boundary
  and `anyhow::Result` at the binary entry points.
- TS/SolidJS: signals only — no class components, no React patterns.
  Path alias `~/` resolves to `ui/src/`.
- Tailwind: use the shared primitives in `ui/src/components/ui/` instead
  of duplicating inline classes. If you need a new primitive, add it there.

## Important docs

- `docs/superpowers/specs/2026-05-17-stint-design.md` — full design spec.
  Always read this before adding a feature; the phase numbering and
  scope decisions live here.
- `docs/superpowers/plans/2026-05-17-stint-phase-1-foundation-cli.md` —
  Phase 1 plan (executed).
- `docs/superpowers/plans/2026-05-18-stint-phase-2-gui.md` — Phase 2 plan
  (executed).
- `README.md` — phase status and user-facing setup.

## Where we are in the roadmap

| Phase | Scope | Status |
|---|---|---|
| 1 | CLI + sync + crash recovery | ✅ shipped (`phase-1-complete`) |
| 2 | Tauri GUI + SolidJS UI | ✅ shipped (`phase-2-complete`) |
| 2.5 | CI baseline (lint / test / typecheck on PR) | ✅ shipped (`phase-2.5-complete`) |
| 3a | OAuth 2.0 foundation + Solidtime OAuth | ✅ shipped (`phase-3a-complete`) |
| 3b | Calendar (Google + MS + CalDAV) | ✅ shipped (`phase-3b-complete`) |
| 3c | Solidtime down-sync | ✅ shipped (`phase-3c-complete`) |
| 3.5 | Test coverage uplift across core / CLI / app / UI | ✅ shipped (`phase-3.5-complete`) |
| 3d | Post-3b UX polish + sync resilience + in-app error surfacing (picker / calendar defaults / editable times / backdate / restart-from-entry / calendar undo / 4xx-abandon / adopt-on-overlap / SyncErrorBanner + coverage CI) | ✅ shipped (`phase-3d-complete`) |
| 4 | Distribution (Homebrew cask + signing + release CD) | planned |
| 5 | Documentation site (GitHub Pages) | planned |

## Gotchas / dev-environment notes

- **Keychain prompts in dev — use `scripts/dev-cli.sh` for CLI.** macOS
  binds Keychain ACL to the binary signature, and clicking "Always Allow"
  stores the binary's exact cdhash — not its designated requirement.
  Every `cargo build` produces a new cdhash, so each rebuild re-prompts
  even when signed by a stable cert. Three-step fix (one-time):
  1. `scripts/setup-dev-cert.sh` — creates a stable self-signed cert
     `stint-dev` in your login keychain. Idempotent.
  2. Use `scripts/dev-cli.sh <subcommand> <args>` instead of
     `cargo run -p stint-cli -- <args>`. The wrapper codesigns with
     `stint-dev` so all rebuilds share the same cert chain.
  3. After the keychain entries exist (you've run `stint config set
     solidtime.token <PAT>`, `scripts/dev-cli.sh config login`, and/or
     `stint calendar add google`), run `scripts/relax-keychain-acl.sh`
     once. It enumerates the Solidtime entries and any calendar account
     entries (discovered via the local SQLite DB), reads each entry's
     password, and re-creates the entry with `security
     add-generic-password -A` (allow any app). The partition-list
     mechanism (`-S codesign:`) was tried first but doesn't grant
     access to self-signed-cert binaries on macOS Sonoma+ — `-A`
     bypasses the cdhash check entirely. Re-run after any new entry
     is created (PAT rotation, new calendar account).
  For the GUI, use `scripts/dev-app.sh` instead of `cargo tauri dev`.
  Same idea as `dev-cli.sh`: it runs `cargo build -p stint-app`,
  codesigns the binary with `stint-dev`, then launches it directly.
  Vite is started in the background on :5173 if it isn't already
  running, so UI HMR still works. The trade-off is Tauri's Rust HMR is
  dropped — for Rust changes, Ctrl+C and re-run. `cargo tauri dev`
  itself can't be wrapped cleanly because it re-invokes `cargo build`
  internally after any pre-launch codesign, undoing the signature
  before launch.
- **Hot reload is flaky.** Vite reliably HMRs the UI. Cargo rebuilds the
  Rust side on save but the Tauri runtime needs to relaunch — sometimes the
  watcher misses changes when many files are touched. When in doubt,
  Ctrl+C and re-run `cargo tauri dev`.
- **The popover window is transparent.** It draws its own rounded card
  inside a 2-px margin to keep shadows visible. Don't add a background
  color to `<body>` for popover windows — the `popover-window` body class
  is set in `App.tsx` precisely to make the body transparent.
- **Two pnpm lockfiles.** Both `pnpm-lock.yaml` (repo root) and
  `ui/pnpm-lock.yaml` exist due to how `pnpm install` ran during Phase 2.
  Don't delete either without testing — the workspace setup expects both.
- **The CLI and GUI share state via SQLite.** A `stint start` in the
  terminal updates the GUI within ~1s (timer store polls). A GUI mutation
  emits the `entries:changed` Tauri event so all UI surfaces refresh
  instantly. CLI-only changes don't emit (only Tauri commands do); the
  polling catches them.
- **`member_id` is required on every time-entry write.** Solidtime returns
  422 without it. We pull it from `solidtime.member_id` in the settings
  table; it's auto-backfilled from the user's memberships when the org is
  picked from the Settings dropdown.
- **Keychain test is env-gated in CI.** `set_get_delete_round_trip` in
  `crates/stint-core/tests/config.rs` honors `STINT_SKIP_KEYCHAIN_TESTS=1`
  and returns early. CI sets it; local dev does not. If you add a new
  test that hits the real Keychain, copy the same three-line guard.
- **Rust toolchain pinned in two places.** `rust-toolchain.toml` pins
  `1.95.0` for local dev; `.github/workflows/ci.yml` pins `1.95.0` for
  CI. Bump both together. (The pin only takes effect locally if you
  invoke cargo via rustup, not via Homebrew-installed rustc, which
  bypasses rustup entirely.)
- **OAuth tokens are one Keychain entry, not three.** Solidtime OAuth
  refresh/access/expiry are persisted as a single JSON blob under
  `tech.reyem.stint.solidtime.oauth`. The blob is rewritten atomically
  on every refresh. The legacy PAT entry at `tech.reyem.stint.solidtime.token`
  is independent — both can coexist; `solidtime.auth_mode` settings key
  picks which is active. The OAuth `client_id` is non-secret and lives
  in the same blob (and is mirrored to the `solidtime.oauth.client_id`
  settings key for first-time setup).
- **OAuth flow needs a registered client on Solidtime.** There's no
  public client-registration UI; users must run `php artisan
  passport:client --public --name=stint --redirect_uri=http://127.0.0.1/callback`
  on their Solidtime host. See the README "Signing in with Solidtime OAuth"
  section for the full setup.
- **Google OAuth credentials are baked at compile time.**
  `crates/stint-core/src/calendar/google/config.rs` reads
  `STINT_GOOGLE_CLIENT_ID` and `STINT_GOOGLE_CLIENT_SECRET` via
  `option_env!`. Set both in the build environment for release builds:

      STINT_GOOGLE_CLIENT_ID=... STINT_GOOGLE_CLIENT_SECRET=... \
        cargo build --release

  Forks that don't set these compile cleanly but Google OAuth fails at
  runtime with `invalid_client`; `is_configured()` returns false and
  the Tauri + CLI surfaces show a clearer error before initiating the
  flow. Credentials live in the build environment (not git) so forkers
  register their own Google Cloud project rather than abusing stint's
  quota.
- **Build-time secrets (Google OAuth) for dev — use `.env.local`.**
  Copy `.env.local.example` to `.env.local` and fill in
  `STINT_GOOGLE_CLIENT_ID` and `STINT_GOOGLE_CLIENT_SECRET`.
  `scripts/dev-app.sh` and `scripts/dev-cli.sh` source `.env.local`
  before invoking cargo so `option_env!` in `stint-core` picks them up.
  `crates/stint-core/build.rs` emits `rerun-if-env-changed` directives
  so changing a value triggers a recompile automatically. Forks without
  `.env.local` build fine but get a "Google OAuth not configured" error
  at runtime when trying to add an account.
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
- **The `verbs::` façade is the single source of truth.** All
  transports (CLI, Tauri commands, HTTP, MCP) delegate to
  `stint_core::verbs::*`. Don't add a new transport without going
  through the façade — duplicating logic at the transport layer is
  how shapes drift.
- **HTTP API is opt-in and loopback-only.** Settings keys:
  `api.enabled` (default `false`), `api.host` (default `127.0.0.1`),
  `api.port` (auto-picked + persisted to the settings table on each
  GUI launch). The server lives inside the running GUI process; the
  CLI does not host it. `stint api info` reads the persisted settings
  and reports the bound URL — useful for scripts that need to discover
  the ephemeral port. Endpoints live under `/v1/`. The model is "the
  trust boundary is anything already running as your user"; no token,
  loopback hard-locked, listener dies when the app quits.
- **MCP server is a CLI subcommand, not a daemon.** `stint mcp` runs
  the rmcp server over stdio. The MCP client (Claude Code, Codex,
  OpenCode) spawns it as a child process — no socket. Install via
  `stint skill install <claude|codex|opencode>`, which calls each
  harness's native registration mechanism (`claude mcp add`, TOML
  merge under `~/.codex/config.toml`, JSON merge under
  `~/.config/opencode/opencode.json`) and drops the bundled SKILL.md
  in the right place per harness.
- **`stint://` URL scheme requires a real `.app` bundle.** macOS
  LaunchServices registers the handler from the bundle's `Info.plist`
  (`CFBundleURLSchemes`). `scripts/dev-app.sh` runs the raw binary
  without a bundle and does NOT register URL handlers. To test deep
  links, run `cargo tauri build` once and let LaunchServices pick up
  the resulting `Stint.app` (or force a re-scan with `lsregister -f
  /Applications/Stint.app`). Supported actions parsed by
  `stint_core::url_scheme`: `stint://start?description=…&project=…&task=…&billable=true`,
  `stint://stop`, `stint://current`, `stint://entry/<local-uuid>`.
- **SKILL.md is the canonical AI-agent guidance.** Lives at
  `crates/stint-cli/skills/stint/SKILL.md` and is `include_str!`-
  bundled into all three harness installers so the same content lands
  regardless of which harness the user picks. Rich content: surface
  ladder (MCP → CLI → HTTP), workflow recipes, project-ID resolution,
  time-math reference, recovery patterns for common Invariant errors.
  Update this file when you add a new tool / verb / behavior — the
  agent learns from it, not from the docs site.
- **`stint generate-man <dir>` emits the man page.** Bundled into
  `Stint.app/Contents/Resources/man/man1/stint.1` at `cargo tauri
  build` time via `beforeBuildCommand`. The Homebrew cask formula in
  `reyemtech/homebrew-tap` needs a `manpage` stanza to expose it to
  `man(1)` on cask installs (separate PR — not landed yet). For
  cargo / `curl|sh` users, `scripts/install-man.sh` is the manual
  path.

## When you start work on a phase

1. Read the design spec section covering it.
2. Use the `superpowers:writing-plans` skill to produce a detailed plan
   doc at `docs/superpowers/plans/YYYY-MM-DD-stint-phase-N-<name>.md`.
3. Branch from `main`: `git checkout -b phase-N`.
4. Execute the plan task-by-task. Commit per task with conventional
   messages.
5. Run the full test suite (`cargo test --workspace -- --test-threads=1`)
   and `pnpm typecheck` before tagging.
6. Open a PR from your phase branch to `main`. Wait for CI to go green.
   Merge via "Create a merge commit" in the GitHub UI. This is the
   project standard because it's the only GH merge mode that both runs
   the change through the PR/CI gate *and* preserves GPG signatures on
   each underlying commit (rebase- and squash-merge re-author commits
   and lose signatures; local fast-forward keeps signatures but
   bypasses the ruleset and skips CI on the merge result). The trade-off
   is a non-linear history — accept the bushiness; the signed history
   matters more here. Then locally fetch, pull `main`, and tag
   `phase-N-complete` and push the tag.

When in doubt about scope, push back rather than build extra.
