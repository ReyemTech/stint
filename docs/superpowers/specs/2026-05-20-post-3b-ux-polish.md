# stint — Post-3b UX polish (spec)

A grouping of UX gaps surfaced during Phase 3b's manual E2E. None are blocking 3b's ship; together they form a coherent "make logging fast and accurate" phase.

- **Status:** Confirmed 2026-05-20
- **Predecessors:** Phase 3b (Google Calendar), Phase 3c (Solidtime down-sync — shipped)
- **Target placement:** Phase 3d. The original "3c — UX polish" placement was reclaimed by the Solidtime down-sync work; this is the next phase after 3c.

## 1. Project picker — searchable + client-grouped

### Problem

The current project picker is a plain `<select>` dropdown that lists every project flatly. With a few dozen projects across multiple clients, scanning for the right one is slow. There's no way to type-to-search; the dropdown reflows whenever the project list refreshes.

### Goal

Replace with a combobox-style picker:

- Click → input + filtered list.
- Type → filter by project name or client name.
- List grouped by client, with the client as a sticky section header inside the popover.
- Selected state shows project + faint client subtitle.
- Same component reused across:
  - Today route's start-timer form
  - Popover's start-timer form
  - Entry edit dialog
  - Calendar "Log this" prefill (when calendar→project mapping is in place — §2)

### Approach sketch

- New primitive in `ui/src/components/ui/ProjectPicker.tsx`.
- Backing data: extend the existing `api.projectsList()` response (or add `api.projectsWithClients()`) to include `client_id` + `client_name` per project.
- Solidtime's projects endpoint already includes `client_id`. We may need a separate `clients` cache that's refreshed alongside projects in the existing Reference sync.
- Keyboard nav: ↑/↓ moves selection, Enter picks, Esc closes.
- Empty state: "No projects match 'foo'."

### Out of scope

- Color/icon per client (Solidtime supports this; defer to v2).
- Multi-select (we only ever pick one project per entry).

## 2. Calendar × project auto-assignment

### Problem

When the user clicks "Log this" on a calendar event today, the resulting time entry has no project. They have to open the entry afterwards and assign one. For users who consistently log a given calendar's events to a given project (standups → "Internal", client calls → "Client A"), this is a repeated chore.

### Goal

Per-calendar default project. When set, "Log this" prefills the project on the new time entry. Easily editable per entry after the fact (the prefill is a suggestion, not a lock).

### Schema

One nullable column on the existing `calendars` table:

```sql
ALTER TABLE calendars ADD COLUMN default_project_id TEXT REFERENCES projects(id) ON DELETE SET NULL;
```

`ON DELETE SET NULL` so a deleted Solidtime project doesn't break the FK; the calendar quietly loses its default.

### UI

- Settings → Calendar accounts → row → "Calendars" dropdown gets a project picker next to each calendar.
- The project picker is the new searchable one from §1.

### Behavior

- `calendar_log_event` (Tauri) reads the calendar's `default_project_id`. If present, passes it to `Entries::create_completed` as `project_id: Some(...)`.
- CLI mirrors via `stint calendar calendars <account_id> --set-project <calendar_id> <project_id>` or similar.
- Setting/clearing the default does NOT retroactively re-assign already-logged entries — those are user-owned data.

### Out of scope

- Multiple rules per calendar ("if event title contains X, project Y"). Deferred to a heavier auto-log subsystem.
- Tag auto-assignment. Same deferral.

## 3. Editable start/end times on entries

### Problem

Once a time entry is logged, its start and end can't be adjusted from the UI. The user has to delete and re-create, losing any sync metadata in the process. CLI `stint edit` already supports description/project changes but not times.

### Goal

A full edit dialog on the GUI that supports:

- Description (already works)
- Project (already works once §1 lands)
- Start time (new)
- End time (new; for completed entries only — running entries' end is undefined)
- Billable toggle (already works)

### Approach

- Time pickers: simple `<input type="time">` plus a date selector that defaults to the entry's start date. Most edits are within the same day; cross-day edits get a separate date field.
- Validation:
  - `end_at > start_at`
  - Duration ≤ 24h (Solidtime's policy; reject otherwise with a clear message)
  - Optional overlap warning with other entries on the same day (don't block — flag with a yellow pill)
- Persist via existing `Entries::update_*` paths; flag `sync_state = 'pending_update'` so the sync queue pushes the change to Solidtime.

### CLI parity

- `stint edit <local_uuid> --start "..."` and `--end "..."` flags. Accept RFC 3339 or HH:MM (defaults to the entry's current date).

### Out of scope

- Recurring entry editing rules (each edit is independent).
- Bulk edits.

## 4. Backdate start when starting a new entry

### Problem

Today, "Start timer" always uses `now` as the start time. Common case the user wants: "I've been working on X for 20 min, log a timer starting then". Currently they start at now, then have to open the just-created entry and rewind start_at (which today they also can't do — see §3).

### Goal

Optional "Start time" override in the start-timer form. Default to now. User can type a HH:MM or pick a time-ago shortcut ("5 min ago", "15 min ago", "30 min ago", "1 hour ago", custom).

### Approach

- Today route + popover start-form get an optional "Start at" field below the description input. Hidden by default behind a "Start later/earlier?" toggle to avoid cluttering the common-case "start now" path.
- New `NewTimeEntry` field: `start_at: Option<String>`. None → now. Some(ts) → that ts.
- `TimerService::start` honours it.
- CLI: `stint start "..." --at "5min ago"` (relative or absolute).

### Validation

- Reject start times in the future (timer can't pre-start).
- Reject start times before any earlier still-running timer (shouldn't happen because only one runs at a time, but defensive).
- Soft-warn on start times more than 12h in the past.

### Out of scope

- Resuming a previously-stopped entry. Different feature (cycle of start/stop), better as its own ticket.

## 5. Phasing

Single phase, four commits + tests:

1. ProjectPicker primitive + reference-sync extension to include clients (§1).
2. `default_project_id` column + migration + Calendars-dropdown UI + log_event prefill (§2).
3. Entry edit dialog with time pickers + CLI flag parity (§3).
4. Backdate start option in start-timer flows + CLI flag (§4).

Estimated total: ~1500 LOC + tests across stint-core (schema + types + commands), stint-app (Tauri commands + UI), stint-cli (subcommand flags), and a meaningful chunk of UI work for the picker.

## 6. Decisions (resolved 2026-05-20)

- **Picker primitive: pull in `@kobalte/core`.** Use its `Combobox` component as the foundation for `ProjectPicker`. Rationale: combobox keyboard nav + ARIA done right is ~200 LOC of fiddly plumbing; @kobalte is Solid-native and well-maintained. This is the first UI library beyond Tailwind primitives — restricted to combobox/menu primitives only; don't reach for it for things tailwind can do.
- **Edit dialog scope: same-day only.** The §3 edit dialog locks the entry's date and exposes only start/end *times*. Cross-day moves remain a delete + re-create workaround. Covers ~95% of edits; revisit if real demand emerges.
- **Backdate shortcuts: four + custom.** "5 min", "15 min", "30 min", "1 hour" buttons plus a custom HH:MM input. Matches the spec's own estimate that these cover ~90% of cases.
