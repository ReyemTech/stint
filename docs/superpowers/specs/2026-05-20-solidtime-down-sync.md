# stint — Solidtime-down sync (spec)

Pull state initiated outside stint back into the local store: running timers, recent entries, and mutations (edits/deletes) made elsewhere.

- **Status:** Draft 2026-05-20
- **Predecessors:** Phase 1 (local-first sync up to Solidtime), Phase 3a (OAuth + PAT auth)
- **Target placement:** Post Phase 3b, as a standalone Phase (provisionally **3.5**)

## 1. Problem

stint today is a one-way pipe. Mutations originate locally (CLI / GUI / menu-bar), persist to SQLite, and drain to Solidtime through the existing `sync_queue` worker. State that originates *outside* stint — a timer started in Solidtime's web UI, an entry edited in another stint instance, a phone client — is invisible to us until we restart with a clean local DB.

Concrete failure mode that motivated this spec: user starts a timer in Solidtime; stint's menu bar shows nothing running; user starts a second timer in stint, ends up with overlapping entries.

## 2. Goals (in priority order)

1. **Running-timer reconciliation** — when stint becomes aware, detect a Solidtime-side active entry and adopt it as the local running timer (or surface a conflict if one is already running locally).
2. **Recent-history reconciliation** — pull completed entries created elsewhere so the Today view, totals, and history match the server.
3. **Mutation reconciliation** — pick up edits and deletes made elsewhere so stale local rows don't outlive the server's state.

(1) is the part that gets immediately felt. (2) and (3) are necessary for the local DB to ever be trustworthy as a source of truth for analytics; without them the user accumulates drift.

## 3. Non-goals (this phase)

- Real-time push from Solidtime (no Pusher / WebSockets / SSE — Solidtime doesn't expose them and we don't need <1s latency).
- Full conflict-free reconciliation à la CRDTs. Local-vs-remote conflicts surface in the UI for human resolution rather than auto-merging.
- Backfilling entries older than the **last 30 days** during normal reconciliation. A separate "Re-import history" admin action covers deeper backfills.
- Multi-user reconciliation. stint is single-user; the spec implicitly assumes the authenticated identity matches one Solidtime member.

## 4. Triggers

The reconciler runs in response to four triggers. All four pull *the same* data; they differ only in cadence and the time window.

| Trigger | When | Window | Reason |
|---|---|---|---|
| `on_startup` | App or CLI starts | last 24h + current running | First-fire wakes a cold cache. Most common path. |
| `on_focus` | Main window gains focus, or CLI command begins (debounced 30s) | last 7d + current running | User explicitly looks at stint; cheap to refresh. |
| `background_poll` | Every 5 minutes while GUI is open | last 1h + current running | Bound the staleness of a stint that's been left open. |
| `manual` | User clicks "Refresh from Solidtime" / CLI `stint pull` | last 30d + current running | Explicit recovery, broader window. |

Network-failure handling: if any trigger fires while offline (DNS/TCP/HTTP), log + skip. Next trigger retries. No exponential backoff state — triggers are cheap and timer-driven anyway.

Triggers and windows are constants in `stint-core::sync::pull::Triggers`, mirroring `calendar::sync::Ranges` (introduced in Phase 3b Task 15). Same shape; same testability.

## 5. Solidtime API surface (read-only)

The reconciler uses three endpoints:

```
GET /api/v1/organizations/{org}/time-entries?member_ids[]={member_id}&start={from}&end={to}
    Returns completed + active entries in the window. `end` is null on the active one.

GET /api/v1/organizations/{org}/time-entries/{id}
    Single entry fetch — used only when we have a solidtime_id locally but the list
    response excluded it (deleted server-side).

GET /api/v1/organizations/{org}/projects   (already used by Reference sync)
GET /api/v1/organizations/{org}/tasks      (already used by Reference sync)
```

Existing `SolidtimeClient` already wraps the org URL pattern; we add two thin methods:

```rust
impl SolidtimeClient {
    pub async fn list_time_entries(&self, member_id: &str, from: &str, to: &str)
        -> Result<Vec<SolidtimeEntry>>;
    pub async fn get_time_entry(&self, id: &str)
        -> Result<Option<SolidtimeEntry>>;   // None on 404
}
```

`SolidtimeEntry` is the existing DTO with one addition: `end: Option<DateTime<Utc>>` to allow the active entry's null end.

## 6. State machine: running-timer adoption

```
remote_running = list response's entry where `end is None` (at most one)
local_running  = running_timer table joined with time_entries

case (remote_running, local_running):

  (None,           None)              → noop
  (None,           Some(local))       → local timer is "extra"; nothing to do here.
                                         The existing sync_queue will push it up
                                         on next drain. Skip.
  (Some(remote),   None)              → ADOPT:
                                         INSERT time_entries (
                                           local_uuid: new,
                                           solidtime_id: remote.id,
                                           description: remote.description,
                                           project_id: remote.project_id,
                                           task_id: remote.task_id,
                                           start_at: remote.start,
                                           end_at: NULL,
                                           billable: remote.billable,
                                           source: 'solidtime',
                                           sync_state: 'synced',
                                           created_at, updated_at: now
                                         )
                                         UPDATE running_timer SET local_uuid = new
                                         EMIT `entries:changed` + `timer:adopted`
  (Some(remote),   Some(local))
    where remote.id == local.solidtime_id
                                       → no-op (already the same timer)
    where remote.id != local.solidtime_id
                                       → CONFLICT (see §7)
```

Identity key: `solidtime_id`. Adoption never creates duplicates because the `time_entries.solidtime_id UNIQUE` constraint catches any second adopt attempt of the same remote entry.

## 7. Conflict policy

When `remote_running != local_running.solidtime_id`, both are "running" in their own world. Default policy: **prefer local, surface remote**.

- **Local timer keeps ticking unchanged.** No interruption to the user's flow.
- **Surface the remote timer in the UI** as a non-blocking banner:
  ```
  Another timer is running in Solidtime:
  "[remote.description]"  started [N] minutes ago
  [Stop it remotely]  [Switch to it]  [Dismiss]
  ```
- **[Stop it remotely]**: enqueue a `stop_remote` op targeting `remote.id`; existing sync queue carries it through.
- **[Switch to it]**: stop the local timer (normal flow → enqueues `update` for end_at), then adopt the remote per §6.
- **[Dismiss]**: the banner is suppressed for this session. The conflict remains until the next trigger; the user has explicitly chosen to live with two parallel timers (probably because the other device is a teammate's, etc.).

Rationale: never silently overwrite the timer the user is actively interacting with. The remote could be from a phone left running, a stale browser tab, an automation — losing the *local* timer (the one the user just looked at) is far more surprising than seeing a banner about the *remote* one.

CLI surfaces the conflict as a warning on the next `stint status` / `stint today` invocation, with the same three actions as flags: `stint pull --stop-remote`, `stint pull --switch`, `stint pull --dismiss`.

## 8. Recent-history reconciliation (goal 2)

For every entry in the list response that does NOT match a local row by `solidtime_id`:

```
SELECT * FROM time_entries WHERE solidtime_id = remote.id
  →  None       → INSERT (source='solidtime', sync_state='synced')
  →  Some(row)
       AND row.sync_state == 'pending_update' or 'pending_delete'
                  → local mutation in flight; leave it alone (queue will push, then
                    next pull picks up the canonical state)
       AND row.sync_state == 'synced' or 'pending_create'
            AND remote.updated_at > row.updated_at
                  → UPDATE local from remote, set sync_state='synced'
            else  → no-op (we already have the latest)
```

`pending_create` overlapping with a remote row only happens if local push hasn't drained yet and the same entry exists upstream — possible if the user manually created it both places. In that case treat the local create as the same entry: take the remote id, clear the queue create op, mark synced. (This is a minor consistency win; sketch the algorithm but don't gold-plate.)

## 9. Mutation reconciliation (goal 3)

Two subcases:

**Remote edit detected** — handled by §8's `updated_at` comparison.

**Remote delete detected** — server returns an entry list without an id we have locally. Reasonable interpretations:
- Entry was deleted on the server. We should mirror that locally.
- Entry simply fell out of the window. We should NOT delete.

Distinguish via the `get_time_entry(id) → Option`. For each local entry with `solidtime_id` and `sync_state = 'synced'` that fell out of the list response's window, fetch by id:
- 404 → delete locally (cascade to entry_tags).
- 200 → entry is still alive, just outside the window. No-op.

This per-missing-entry fetch is O(N) but N is small (entries with `solidtime_id` AND `start_at` inside the window AND missing from the list = probably 0 in steady state). Cap at 50 per pull to bound worst-case cost; rest defer to the next trigger.

## 10. Data flow summary

```
trigger fires
   │
   ▼
list_time_entries(member_id, from, to)   ←─ existing SolidtimeClient extension
   │
   ▼
reconcile_running(remote_active, local_running)         → §6 + §7
   │
   ▼
reconcile_history(remote_completed_list, local_window)  → §8
   │
   ▼
reconcile_deletes(local_synced_in_window_not_in_list)   → §9
   │
   ▼
emit `entries:changed` (single event per pull, debounced)
```

All three steps share the same transaction at the SQLite layer so partial application doesn't leave the local DB inconsistent.

## 11. Schema

No new tables. Two small additions:

- **`time_entries.source`** already exists; new valid value: `'solidtime'` (alongside `'cli'`, `'gui'`, `'calendar'`). Strictly a string column with no constraint; nothing to migrate, just to document.
- **`time_entries` indexes**: confirm `idx_time_entries_solidtime_id` exists for fast `WHERE solidtime_id = ?` lookups. If not, add via migration `0003_time_entries_solidtime_id_index.sql`.

The `source = 'solidtime'` value is a signal for the UI to render a small Solidtime icon next to the entry, distinguishing remote-origin from local-origin entries.

## 12. UI

**Menu bar popover:**
- When adoption fires on app startup with no local timer, the popover updates within a tick to show the adopted timer running. Visual: same as a locally-started timer, plus a "synced from Solidtime" subscript.

**Main window:**
- A "Last synced N seconds ago" line under the Today header. Click to manually refresh (manual trigger).
- Conflict banner per §7. Persists until dismissed or resolved.

**CLI:**
- `stint status` prints "(adopted from Solidtime)" on the running-timer line if `source = 'solidtime'`.
- `stint pull` new subcommand triggers manual reconciliation. Prints "+N entries, ~M updates, -K deletes" summary.

## 13. Failure modes

- **Network offline at trigger**: skip, no error UI. Next trigger retries.
- **HTTP 401 on the list call**: re-auth via existing OAuthTokenProvider refresh; if refresh fails, surface via the existing auth-failure banner.
- **HTTP 5xx**: log a warning, skip. Next trigger retries. No partial state because the reconcile transaction commits atomically at the end.
- **Solidtime returns malformed JSON for an entry**: log a warning, skip that one entry, continue with the rest. Don't abort the whole pull on a single bad row.
- **Local DB write fails mid-reconcile**: transaction rolls back; no observable state change. Trigger retries.
- **User has two stint instances on different machines**: both reconcile from the same server independently. The last-writer-wins via `updated_at` comparison in §8 is correct for both.

## 14. Test plan

- **TDD against a mock `SolidtimeClient`**: feed scripted list responses, assert local DB transitions match §6/§8/§9 tables.
- **Wiremock-based integration**: verify the real HTTP request shape (query params, headers) matches what Solidtime expects.
- **Conflict UI**: manual click-through with two browser tabs (one local stint, one Solidtime web).
- **Failure injection**: 401, 500, network drop, malformed response.

## 15. Out-of-scope, deferred

- Pulling entry comments / attachments (Solidtime doesn't expose these yet on entries).
- Pulling tag changes — tags are a separate sub-API, addressable in a follow-up.
- Push side of "switch to remote": currently `[Stop it remotely]` enqueues a stop op. A "Start remote" action (start a new timer on Solidtime from stint) would close the loop but isn't motivated by any reported workflow.

## 16. Estimated phasing

The three goals are independent enough to ship in three commits, each gated by its own tests:

1. **Adoption + conflict UI** — covers the reported pain (running timer). ~400 LOC + tests.
2. **Recent-history reconciliation** — adds the SELECT/INSERT/UPDATE branches of §8. ~300 LOC + tests.
3. **Delete reconciliation** — adds the per-missing-id fetch and local delete. ~200 LOC + tests.

The trigger framework, the `SolidtimeClient` extensions, and the UI plumbing land with (1) and are reused. Total ~1000 LOC across a single phase.
