---
title: CLI commands
description: Complete reference for every stint CLI subcommand.
---

`stint --help` lists the full set. This page documents each subcommand
with its actual flag names + positional arguments. If anything here
disagrees with `stint <cmd> --help`, the help output is the source of
truth — file an issue and we'll fix the page.

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
