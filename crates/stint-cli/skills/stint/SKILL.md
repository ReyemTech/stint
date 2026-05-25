---
name: stint
description: Use when the user wants to start/stop/edit a time entry, log work to stint, ask what they're currently tracking, query past entries, or list projects/tasks. stint is a macOS time tracker syncing to Solidtime; this skill drives it via MCP tools (preferred), CLI `stint --json`, or HTTP API as fallbacks.
---

# stint

stint is the user's macOS time tracker. Entries persist locally in SQLite and sync to a self-hosted Solidtime instance in the background.

## Surface priority — try in this order

You have up to three ways to talk to stint. Use the highest one that works.

1. **MCP tools** (preferred — typed schemas, lowest latency, no shell)
   - 8 tools exposed by the `stint` MCP server: `start`, `stop`, `current`, `list_entries`, `list_projects`, `list_tasks`, `update_entry`, `delete_entry`
   - If the server isn't connected, the tools won't appear in your tool list — drop to surface 2.

2. **CLI `stint --json`** (always works if `stint` is in PATH)
   - Every verb supports `--json`: `stint --json current`, `stint --json start "writing tests"`, `stint --json list --limit 10`, etc.
   - Returns the same JSON shapes as the MCP tools.
   - Use this when MCP is unavailable or when you need a verb the MCP server doesn't expose (admin verbs: `sync`, `pull`, `config`, `calendar`, `update`).
   - Discover where stint lives: `which stint`. If missing, try `~/.cargo/bin/stint` or the Stint.app bundle's `Contents/MacOS/stint`.
   - **Discoverability**: if you need a flag or subcommand you don't remember, use:
     - `stint --help` — top-level commands list
     - `stint <verb> --help` — flags for a specific verb (e.g., `stint start --help`)
     - `man stint` — full reference page (if installed; see `stint generate-man --help` for local install)

     These are the source of truth for the CLI surface. The tool descriptions in this skill are a curated subset.

3. **HTTP API** (loopback, only when GUI is running)
   - Discover the URL: `stint --json api info` → `{ "enabled": …, "port": …, "base_url": "http://127.0.0.1:54321" }`
   - Endpoints: `POST /v1/start`, `POST /v1/stop`, `GET /v1/current`, `GET /v1/entries?since=…&until=…&project_id=…&limit=N`, `GET /v1/projects`, `GET /v1/tasks?project_id=…`, `PATCH /v1/entries/:id`, `DELETE /v1/entries/:id`.
   - If `api.enabled` is false or the port is unreachable, the GUI is closed — fall back to CLI.

**Pick a surface and stick with it within a single user request** to avoid mixing read/write paths.

### Bonus surfaces (Phase 6b — user-facing, agent-aware)

These are macOS shell surfaces. Agents don't invoke them directly, but should know they exist when answering questions about how the user works with stint:

- **App Intents in Shortcuts.app + Siri** — 5 App Shortcuts (Start Timer, Stop Timer, Current Timer, Switch Project, Log Past Work) callable via voice ("Hey Siri, start tracking in Stint") and Spotlight quick actions. All 8 verbs + 2 composed (SwitchProject, LogPast) are discoverable as Custom Shortcuts.
- **Core Spotlight** — entries, projects, and tasks are indexed. Cmd+Space → "client meeting" → tap → opens the entry. Cmd+Space → "Acme" → tap → opens stint filtered to that project.
- **macOS Focus filter** — `System Settings → Focus → <mode> → Add Filter → Stint → Default Project`. While that focus is active, new `stint start` calls without an explicit project pick up the Focus-defaulted project. **Race window:** if the user activates a focus while Stint.app is cold-launching, the default may not have been written yet — the next `stint start` will record the entry without the project, fixable via `stint edit`.
- **stint:// URL routes** (additions for 6b):
  - `stint://project/<solidtime_id>` → opens Today view filtered to the project.
  - `stint://task/<solidtime_id>` → resolves task → parent project, filters by both.
  - Existing: `stint://start?description=…&project=…`, `stint://stop`, `stint://current`, `stint://entry/<local_uuid>`.

## When to use this skill

Triggers (not exhaustive):

- "Start a timer for X" / "I'm working on X"
- "Stop the timer" / "I'm done"
- "What am I working on?" / "Is anything running?"
- "Log 30 minutes for X" / "I forgot to start — last hour was X"
- "How much did I spend on project Y this week?" / "Show today's entries"
- "Switch to project Z" (stop current, start new)
- "Resume what I was working on yesterday"
- "List my projects" / "What projects do I have?"
- "Delete that entry" / "Remove the last one"

## Workflow recipes

### Start a timer (simple case)
1. `current` to check nothing's running (start errors if a timer is already active).
2. If running, ask the user whether to stop the current one first.
3. `start { description, project_id?, task_id?, billable? }`. The `source` field is auto-set to `"mcp"` (or `"cli"`/`"http"`).

### Switch projects
1. `current` → returns the running entry.
2. `stop`.
3. Resolve target project_id (see "Project ID resolution" below).
4. `start { description, project_id }`.

### Log a meeting that just happened
Prefer `update_entry` over `start` with backdated `start_at` when possible — keeps the timer-running model clean. If no existing entry to update:
1. `current` — if a timer is running, stop it first (otherwise the new backdated entry will be rejected when it overlaps the running one).
2. `start { description, start_at: <ISO 8601 UTC>, project_id? }`.
3. Immediately `stop`. The entry now has correct start_at + end_at = now.
4. Optional `update_entry` to set end_at to a specific moment.

### Resume yesterday's work
1. `list_entries { since: "<yesterday UTC midnight>", until: "<today UTC midnight>", limit: 20 }`.
2. Find the most recent (highest start_at) entry matching what the user described.
3. `start { description: that.description, project_id: that.project_id, task_id: that.task_id, billable: that.billable }`.

### "How much on project X this week?"
1. `list_projects` and fuzzy-match X to a project name → project_id.
2. `list_entries { since: <Monday UTC midnight>, until: <next Monday UTC midnight>, project_id }`.
3. Sum durations from each entry's `start_at` to `end_at` (skip the running entry, where `end_at` is null).

### Edit an entry's project/task/billable
Use `update_entry` with `EntryPatch`. The patch fields support a 3-way distinction for nullable fields:
- Field absent (omitted from JSON) = no change
- Field present with `null` = clear the field
- Field present with a value = set to that value

```json
{ "description": "new desc", "project_id": null }   // change desc, clear project
{ "billable": true }                                  // toggle billable on, leave rest
```

### Stop and discard (delete in one step)
1. `current` to get the local_uuid.
2. `stop`.
3. `delete_entry { local_uuid }`.

## Project / task ID resolution

Users say "the auth project", "feature X", "PR review" — they don't say `01HPYJK…`. Resolve names → IDs:

1. Call `list_projects` once at the start of the session, cache the result.
2. Fuzzy-match the user's words against `project.name` (case-insensitive substring is usually fine).
3. If multiple matches, ask the user.
4. Use `project.solidtime_id` (a UUID) for `project_id` in subsequent calls.

Tasks: `list_tasks { project_id }` to scope the lookup. Most users don't reference tasks by name often.

If a `start` returns "project_id not found", the project may be archived or not yet pulled from Solidtime — suggest `stint pull` to refresh reference data.

## Time math reference

stint stores timestamps as ISO 8601 UTC strings. The user thinks in local time, but the API wants UTC.

| Wanted | Compute |
|---|---|
| Today (UTC midnight to next midnight) | `since = floor(now to UTC midnight)`, `until = since + 24h` |
| This week (Mon UTC midnight to next Mon) | floor `now` to UTC midnight, subtract `(weekday - 1) days` |
| Last 24h | `since = now - 24h`, `until = now` |
| Yesterday | `since = today_midnight - 24h`, `until = today_midnight` |
| ISO 8601 UTC format | `2026-05-24T03:15:00Z` (Z suffix, no offset) |

**Be careful**: the user's "today" is their local timezone. A 9 PM PT entry is the next day in UTC. If the user says "today", use *their* local midnight as the window — convert to UTC for the API call. Default to UTC if you don't know their timezone.

## Common failure modes + recovery

| Error from MCP / CLI / HTTP | What it means | Recovery |
|---|---|---|
| MCP tool returns `Invariant: timer already running` | `start` rejected because a timer is active | Call `current`, ask user to stop or extend |
| MCP tool returns `Invariant: cannot set start_at on a running entry without also setting end_at` | `update_entry` on a running entry tried to change times | Stop the entry first, then update |
| MCP tool returns `NotFound: entry <uuid>` | The local_uuid doesn't match any entry | Re-fetch via `list_entries` or `current` |
| `stint mcp` is in the MCP server list but tools don't appear | Server crashed or is wedged | Drop to CLI: `stint --json <verb>` |
| `command not found: stint` | Not in PATH | Try `~/.cargo/bin/stint`, the Stint.app bundle, or `/opt/homebrew/bin/stint` |
| `stint api info` shows `enabled: false` | HTTP API is off | Run `stint config set api.enabled true` then restart the GUI |
| `curl http://127.0.0.1:<port>/v1/...` returns connection refused | GUI is not running | Fall back to CLI; or launch the app and re-discover port |
| HTTP `404` on a known-good endpoint | App restarted, port changed | Re-run `stint --json api info` for the current port |
| Solidtime sync returns 422 | Usually missing `member_id` setting | Surface the error and suggest the user verify Solidtime configuration in Settings |

## What NOT to do

- **Don't poll `current` in a loop.** It hits SQLite on every call. If you need to watch for changes, ask the user to ping you again later.
- **Don't `delete_entry` without confirming.** Soft-deletes are recoverable from sync, but the user should approve any destructive action by default.
- **Don't backdate beyond 24 hours.** `update_times` validates that any single entry covers ≤24h and that end > start. Solidtime also rejects entries older than its own retention window.
- **Don't invent project_id or task_id values.** Always resolve from `list_projects` / `list_tasks`. A wrong UUID will be rejected by Solidtime sync silently (the entry persists locally but never syncs).
- **Don't bypass the MCP server by writing directly to the SQLite file.** Use CLI or HTTP — they handle sync queue ops correctly.
- **Don't enable HTTP API casually.** It binds to 127.0.0.1 only, but users may not want any local listener. Ask before suggesting `api.enabled = true`.
- **Don't fabricate CLI flags or verbs.** If a verb or `--flag` you're using gets "unrecognized" or "unknown argument" errors, run `stint <verb> --help` or `stint --help` to see the real surface — don't keep guessing.
