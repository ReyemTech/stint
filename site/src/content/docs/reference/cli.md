---
title: CLI commands
description: Complete reference for every stint CLI subcommand.
---

`stint --help` lists the full set. This page documents each subcommand with
typical usage and notable flags.

## Timer

### `stint start <description> [options]`

Starts a new timer. Stops any running timer first.

```bash
stint start "fixing the queue bug"
stint start "writing tests" --project <project-uuid>
stint start "client call" --billable
stint start "started at lunch" --start-at "2026-05-23T12:30:00-04:00"
```

Flags:
- `--project <uuid>` — link the entry to a project (must exist in Solidtime; refresh with `stint projects refresh`)
- `--billable` — mark billable; otherwise inherits the project's default
- `--start-at <iso8601>` — backdate the start time

### `stint stop`

Stops the running timer. No-op if none running.

### `stint restart <entry-id>`

Starts a new timer with the same description / project / billable status
as the named entry. Useful for "do that again, please".

### `stint today`

Shows today's entries, durations, and sync state.

### `stint list --from <date> --to <date>`

Lists entries in a date range.

### `stint edit <entry-id> [--description …] [--start …] [--end …]`

Edits an existing entry. Synced entries are re-queued for update on save.

### `stint delete <entry-id>`

Deletes an entry locally; the deletion syncs to Solidtime on next drain.

## Configuration

### `stint config show`

Prints non-secret configuration (URL, org, member ID, channels, etc.).

### `stint config set <key> <value>`

Sets a configuration value. Secret keys (`solidtime.token`,
`solidtime.oauth.*`) write to the macOS Keychain instead of the SQLite
settings table.

### `stint config test`

Round-trip API call to Solidtime. Reports the authenticated user + org if
successful.

### `stint config login`

Initiates the Solidtime OAuth flow (browser-based). Requires
`solidtime.oauth.client_id` to be set first.

### `stint config logout`

Clears OAuth tokens from the Keychain. The PAT (if any) is unaffected.

## Projects

### `stint projects list`

Lists projects mirrored from Solidtime.

### `stint projects refresh`

Pulls the latest projects, tasks, and tags from Solidtime into the local
cache. Run after creating new projects in Solidtime.

### `stint projects orgs`

Lists organizations the authenticated user belongs to.

## Sync

### `stint sync` (alias: `stint sync drain`)

Drains the sync queue immediately. The background worker drains every 30s
when the app is running; this command forces it.

### `stint sync retry-abandoned`

Re-enqueues entries marked as abandoned (4xx response from Solidtime that
the worker stopped retrying). Useful after fixing a server-side issue.

### `stint pull`

Pulls recent server-side state (running timers on other devices, edits to
recent entries). The GUI runs this on startup and every 5 minutes.

## Calendar

### `stint calendar add google`

Initiates Google Calendar OAuth (browser-based). Stores tokens in the
Keychain per account.

### `stint calendar list`

Lists connected calendar accounts.

### `stint calendar list-calendars [--account <id>]`

Lists calendars under each account.

### `stint calendar set-included <calendar-id> --included true|false`

Toggles whether a calendar surfaces events in the GUI's event picker.

### `stint calendar set-default-project <calendar-id> --project <uuid>`

Sets the project that calendar-logged entries default to for the given
calendar.

### `stint calendar refresh [--account <id>]`

Forces a calendar event refresh.

### `stint calendar remove <account-id>`

Removes a calendar account. Deletes both the database row and the Keychain
entry.

## Update

### `stint update`

For standalone CLI installs: downloads and applies the latest release.
Verifies checksum + atomically swaps the binary.

For `.app`-bundled installs (brew, `curl | sh --gui`): prints a hint
pointing at the proper update channel (`brew upgrade --cask stint` or the
in-app Settings → Updates panel) — does not self-update.

### `stint update --check`

Reports the available version without applying.

## Help

### `stint help [command]`

`stint help` lists all commands. `stint help <command>` shows the
detailed help for a single command. Every subcommand also accepts
`-h` / `--help` directly.
