# CLAUDE.md — Repo notes for AI coding agents

This file is the entry point for any AI coding assistant (Claude Code,
Cursor, Codex, etc.) working in this repository. Read it before touching code.

A duplicate `AGENTS.md` exists for non-Claude agents; both files point to the
same content — keep them in sync.

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
- `phase-N` — feature branches; merged fast-forward to `main` once tagged
- `phase-N-complete` — annotated tag marking each phase's release

For new phases, branch from `main`:

```bash
git checkout -b phase-2.5
```

### Testing discipline

- **TDD for `stint-core`** — write the failing test, then the implementation.
  See any of `crates/stint-core/tests/*.rs` for the pattern.
- **Integration over unit** for store-level code — tests run against a real
  SQLite tempdir via `tests/common/mod.rs` `setup()`.
- **Wiremock for HTTP** — `crates/stint-core/tests/solidtime.rs` and
  `sync_*` files show the pattern.
- **`assert_cmd` for CLI** — `crates/stint-cli/tests/cli_e2e.rs`.
- **For UI**: no automated tests yet. Manual visual verification only —
  `cargo tauri dev` and click through every route after a change.
  Compile-pass is NOT proof the UI works.

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
| 3b | Calendar (Google + MS + CalDAV) | planned |
| 4 | Distribution (Homebrew cask + signing + release CD) | planned |
| 5 | Documentation site (GitHub Pages) | planned |

## Gotchas / dev-environment notes

- **Keychain prompts in dev — use `scripts/dev-cli.sh` for CLI.** macOS
  binds Keychain ACL to the binary signature. Plain `cargo build` produces
  a different signature on every rebuild, so "Always Allow" never sticks.
  Fix: run `scripts/setup-dev-cert.sh` once to create a stable self-signed
  cert named `stint-dev` in your login keychain. Then run CLI dev work via
  `scripts/dev-cli.sh <subcommand> <args>` instead of `cargo run -p stint-cli`.
  The wrapper builds, codesigns with `stint-dev`, then execs the binary —
  same signature every time, so "Always Allow" persists.
  GUI (`cargo tauri dev`) is NOT yet covered by this; Tauri spawns the
  binary itself outside of cargo run, so wrapping it requires a custom
  file-watcher. Left as a follow-up.
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
   Merge via "Rebase and merge" in the GitHub UI (preserves linear
   history equivalent to a fast-forward). Then locally fetch, pull
   `main`, and tag `phase-N-complete` and push the tag.

When in doubt about scope, push back rather than build extra.
