---
name: stint
description: Use when the user wants to start/stop a time entry, log work to stint, or query their current timer. Talks to the stint time tracker via MCP tools (start, stop, current, list_entries, list_projects, list_tasks, update_entry, delete_entry).
---

# stint

stint is the user's macOS time tracker. It syncs with a self-hosted Solidtime instance.

## When to use

- "Start a timer for X" → call `start` with `description`
- "What am I working on?" → call `current`
- "Stop the timer" → call `stop`
- "List today's entries" → call `list_entries` with `since` set to today's UTC midnight
- "Show my projects" → call `list_projects`

## Discipline

- **Always check `current` before calling `start`** — start errors if a timer is already running, and the user usually wants the running timer either stopped or extended, not a parallel start.
- When the user says "log 30m for X", prefer `update_entry` on an existing entry over creating a new one with backdated `start_at`.
- The `source` field on entries created via MCP is automatically set to `"mcp"`.
