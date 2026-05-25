---
title: CLI commands
description: Complete reference for every stint CLI subcommand.
---

`stint --help` lists the full set. This page documents each subcommand
with its actual flag names + positional arguments. If anything here
disagrees with `stint <cmd> --help`, the help output is the source of
truth — file an issue and we'll fix the page.

## Discoverability

Three places double as the canonical CLI reference — use whichever you
have handy:

- `stint --help` — every top-level subcommand
- `stint <verb> --help` (or `-h`) — flags for one verb
  (e.g. `stint start --help`)
- `man stint` — the full man page; installed by the Homebrew cask, or
  via `scripts/install-man.sh` for cargo / `curl|sh` installs

## Global flags

### `--json`

Every CLI command accepts a global `--json` flag (placed before or after
the subcommand). It swaps human-readable text for machine-readable JSON
on stdout. Errors still write a structured message to stderr and exit
non-zero.

```bash
stint --json current
# null   — or { "local_uuid": …, "description": …, … }

stint --json start "writing tests" --project p-uuid
# { "local_uuid": "…", "description": "writing tests", … }

stint --json sync
# { "refreshed": true }
```

The output convention:

- **Read verbs** (`current`, `today`, `list`, `projects list`,
  `api info`, `skill status`, …) emit the canonical view type (entry,
  list of entries, info struct).
- **Write verbs** (`start`, `stop`, `edit`, `restart`, `delete`) emit
  the affected entry as JSON.
- **Admin verbs** (`sync`, `pull`, `config set/show/test/login/logout`,
  `calendar *`, `update`, `skill install/uninstall`) emit a structured
  acknowledgement like `{ "refreshed": true }` or
  `{ "uninstalled": true, "harness": "claude" }`.

JSON output makes stint scriptable from any language with a JSON parser.
See [Scripting stint](/scripting/) for end-to-end examples.

## Timer

### `stint start <description> [--project <uuid>] [--task <uuid>] [--at <when>]`

Starts a new timer. Stops any running timer first.

```bash
stint start "fixing the queue bug"
stint start "writing tests" --project <project-uuid>
stint start "started at lunch" --at "12:30"
stint start "did this an hour ago" --at "1h ago"
```

Flags:
- `--project <uuid>` — link the entry to a project (must exist in
  Solidtime; refresh the local cache with `stint projects refresh`)
- `--task <uuid>` — link the entry to a task under that project
- `--at <when>` — backdate the start. Accepts:
  - Relative: `"15min ago"`, `"1h ago"`
  - Bare `HH:MM` (local time today; shifted to yesterday if in the future)
  - RFC 3339: `2026-05-23T12:30:00-04:00`

Billable status inherits from the project's default; it isn't a CLI
flag (use `stint edit --description …` and the GUI to override per
entry).

### `stint stop`

Stops the running timer. Errors if none is running.

### `stint current`

Prints the currently running entry (description, project, start time,
elapsed). With `--json`, emits the entry object or `null` when nothing
is running.

```bash
stint current
# → writing tests · 0:14 elapsed · started 14:02

stint --json current
# → null   (or the entry object)
```

### `stint restart <local-uuid>`

Starts a new timer cloning an existing entry's description, project,
task, and billable flag. Stops any running timer first.

```bash
stint restart abc12345
```

### `stint today`

Shows today's entries, durations, and sync state.

### `stint list <from> <to>`

Lists entries in a UTC ISO-8601 date range. Both arguments are
**positional, not flags**.

```bash
stint list 2026-05-20T00:00:00Z 2026-05-23T23:59:59Z
```

### `stint edit <id> [--description …] [--start HH:MM] [--end HH:MM]`

Edits an existing entry. `<id>` accepts the full UUID or its 8-character
prefix. `--start` and `--end` interpret as local time on the entry's
existing date.

```bash
stint edit abc12345 --description "different name for this work"
stint edit abc12345 --start 09:15 --end 10:30
```

Synced entries are re-queued for update on save. Running entries can't
have their times edited (only completed entries).

### `stint delete <id>`

Deletes an entry locally; the deletion syncs to Solidtime on next drain.

```bash
stint delete abc12345
```

## Configuration

### `stint config set <key> [<value>]`

Sets a configuration value. For the secret key `solidtime.token`, omit
the value to be prompted (the token then goes to the macOS Keychain
instead of the SQLite settings table).

```bash
stint config set solidtime.url https://your-host.example.com
stint config set solidtime.org <organization-uuid>
stint config set solidtime.token                 # prompts; → Keychain
```

### `stint config show`

Prints all non-secret settings. Token values are masked.

### `stint config test`

Round-trip API call to Solidtime. Reports the authenticated user + org
if successful.

### `stint config login`

Initiates the Solidtime OAuth flow (browser-based). Requires
`solidtime.oauth.client_id` to be set first.

### `stint config logout`

Removes the OAuth token blob from Keychain. The PAT (if any) is
unaffected.

## Projects

### `stint projects list`

Lists projects mirrored from Solidtime in the local cache.

### `stint projects refresh`

Pulls the latest projects, tasks, and tags from Solidtime into the cache.
Run after creating new projects in the Solidtime UI.

### `stint projects raw`

Prints the raw Solidtime `/projects` response. Diagnostic only.

## Sync

### `stint sync` (no subcommand, defaults to `drain`)

Drains the sync queue once. The background worker drains every 30s when
the GUI is running; this command forces it immediately from the CLI.

### `stint sync drain`

Explicit form of the default. Same as `stint sync`.

### `stint sync retry-abandoned`

Resurrects queue rows previously parked far in the future by the
abandon-on-4xx path (typically after a fix-forward release that resolved
the 4xx root cause). Their attempts counter resets so the worker gives
the new code a fresh try.

### `stint sync force-adopt <local-uuid> <remote-id>`

Manually links a local pending-create entry to an existing remote ID.
For unsticking entries where adopt-on-overlap couldn't auto-resolve
(e.g. the remote's start time differs from local). Drops any queued
create_entry op for this UUID.

### `stint sync active`

Prints every currently-running entry Solidtime sees for the configured
member, regardless of project filter. Diagnostic — useful when overlap
rejections happen but the Solidtime web UI doesn't show what's
blocking.

### `stint sync diagnose <local-uuid>`

Dumps every Solidtime entry whose time range intersects the local
entry's `[start, end]` — the real overlap set. Solidtime forbids any
range overlap (running OR completed), so a stuck `overlap` 400 with no
visible active entry usually means a completed entry is the actual
blocker.

## Pull

### `stint pull [--dismiss | --stop-remote | --switch]`

Pulls recent Solidtime state (running timers on other devices, edits to
recent entries). The GUI runs this on startup and every 5 minutes.

If a conflict is detected (another device has a running timer), the
flag controls resolution:

- `--dismiss` (default behavior) — surface the conflict without resolving
- `--stop-remote` — stop the remote running timer
- `--switch` — stop the local timer and adopt the remote one

The flags are mutually exclusive.

## Calendar

### `stint calendar add <provider>`

Initiates calendar OAuth (browser-based). Currently only `google` is
accepted.

```bash
stint calendar add google
```

Stores OAuth tokens in the Keychain, one entry per account.

### `stint calendar list`

Lists connected calendar accounts.

### `stint calendar remove <account-id>`

Removes a calendar account. Deletes both the database row and the
Keychain entry.

### `stint calendar calendars <account-id> [flags]`

Lists or modifies calendars under an account. With no flags, prints
each calendar with its inclusion state and any default project.

```bash
stint calendar calendars <account-id>
```

Modification flags (any combination):

- `--include <calendar-id>` — mark a calendar as included (events
  appear in the GUI's event picker)
- `--exclude <calendar-id>` — mark a calendar as excluded
- `--set-default-project <CALENDAR_ID> <PROJECT_ID>` — set the default
  project that calendar-logged entries land under (takes two values)
- `--clear-default-project <calendar-id>` — clear the default project

```bash
stint calendar calendars acc-uuid --include cal-id
stint calendar calendars acc-uuid --set-default-project cal-id proj-uuid
```

### `stint calendar refresh <account-id>`

Forces a refresh of that account's events (using the
on-focus window).

```bash
stint calendar refresh acc-uuid
```

## HTTP API

### `stint api info`

Reports the loopback HTTP API's state — whether it's enabled, the bound
host (defaults to `127.0.0.1`), the ephemeral port the GUI picked, and
the resolved base URL. The CLI does not start the server itself; the GUI
process hosts it.

```bash
stint --json api info
# { "enabled": true, "host": "127.0.0.1", "port": 54321,
#   "base_url": "http://127.0.0.1:54321" }
```

Enable the API with `stint config set api.enabled true` (or via Settings
→ Integrations) and (re)start the GUI. See [Scripting stint](/scripting/)
for endpoint details.

## AI / MCP

### `stint mcp`

Runs stint as an [MCP](https://modelcontextprotocol.io/) server over
stdio. MCP-aware clients (Claude Code, Codex, OpenCode, …) spawn this as
a child process — you don't typically invoke it directly. Exposes 8
tools (`start`, `stop`, `current`, `list_entries`, `list_projects`,
`list_tasks`, `update_entry`, `delete_entry`).

See [AI integration](/ai-integration/) for setup.

### `stint skill install [<harness>] [--dry-run]`

Wires stint's MCP server + the bundled `SKILL.md` into an AI harness's
config. Without `<harness>`, opens an interactive picker. With
`--dry-run`, prints what would change without writing files.

```bash
stint skill install              # interactive picker
stint skill install claude       # explicit
stint skill install codex --dry-run
```

Supported harnesses: `claude` (Claude Code), `codex` (OpenAI Codex CLI),
`opencode`.

### `stint skill uninstall [<harness>]`

Reverses an install. Removes both the MCP registration and the skill /
rules fragment. `.bak` backups of any mutated config files are
preserved.

### `stint skill status`

Reports per-harness install state.

```text
Claude Code     detected=true   mcp=true   skill=true
Codex CLI       detected=false  mcp=false  skill=false
OpenCode        detected=true   mcp=true   skill=true
```

See [AI integration](/ai-integration/) for the full integration story.

### `stint generate-man <out-dir>`

Writes the `stint.1` man page into `<out-dir>`. Used by the Tauri build
to bundle the man page inside `Stint.app/Contents/Resources/man/`, and
by `scripts/install-man.sh` to install it system-wide for
`cargo install` / `curl|sh` users.

```bash
stint generate-man /tmp
man /tmp/stint.1
```

## `stint://` URL scheme

Stint.app registers the `stint://` URL scheme on macOS so other tools
(Raycast, Alfred, Shortcuts, browser bookmarklets, scripts) can deep-
link into the app. Use `open` to dispatch a URL:

| URL shape | Effect |
|---|---|
| `stint://start?description=…&project=<uuid>&task=<uuid>&billable=true` | Starts a timer (description required; project/task/billable optional) |
| `stint://stop` | Stops the running timer |
| `stint://current` | Focuses the current-entry view |
| `stint://entry/<local-uuid>` | Opens the given entry in the main window |

Examples:

```bash
open "stint://start?description=writing%20tests"
open "stint://start?description=fix%20bug&project=abc-123&billable=true"
open "stint://stop"
open "stint://entry/01HPYJK..."
```

Standard percent-encoding rules apply (`%20` for spaces, `+` also
decodes to space). The scheme is only registered when `Stint.app` is
installed via DMG / brew / `cargo tauri build`; raw `cargo run` builds
don't register the handler. See
[Troubleshooting](/help/troubleshooting/) if `stint://` URLs don't open.

## Update

### `stint update [--check] [--force]`

For **standalone CLI installs** (via `curl | sh` without `--gui`):
downloads and applies the latest release. Verifies checksum + atomically
swaps the binary.

For **`.app`-bundled installs** (brew, `curl | sh --gui`): prints a
hint pointing at the proper update channel (`brew upgrade --cask stint`
or the in-app **Settings → Updates** panel). Does not self-update — the
GUI manages its own updates.

Flags:

- `--check` — report the available version without applying
- `--force` — apply even if already on the latest version

## Help

`stint help` lists every subcommand. `stint help <command>` shows the
detailed help for one. Every subcommand also accepts `-h` / `--help`
directly.
