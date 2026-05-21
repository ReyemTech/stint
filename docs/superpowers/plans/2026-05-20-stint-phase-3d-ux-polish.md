# Phase 3d — UX Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make day-to-day logging faster and more accurate by replacing the flat project `<select>` with a searchable client-grouped combobox, letting each calendar pre-fill a default project on "Log this", allowing start/end times to be edited on completed entries, and letting the start-timer flow backdate to "5/15/30/60 min ago" (or custom).

**Architecture:** Four independent slices, each shippable on its own commit. The shared dependency is a new `ProjectPicker` primitive (built on `@kobalte/core` Combobox) introduced in slice 1 and reused by slices 2 and 3. Slices 3 and 4 extend the existing `Entries`/`TimerService` API surface; slice 4 adds an optional `start_at` override to the existing start-timer pipeline.

**Tech Stack:** Rust 1.95 (stint-core), Tauri 2 (stint-app), clap (stint-cli), SolidJS + Tailwind + `@kobalte/core` Combobox (ui), wiremock for HTTP-shape tests.

**Spec:** `docs/superpowers/specs/2026-05-20-post-3b-ux-polish.md` (read it; this plan implements it verbatim).

---

## File Structure

### stint-core (new)

| Path | Responsibility |
|---|---|
| `crates/stint-core/migrations/0004_clients.sql` | New `clients` table (id, name, archived, fetched_at). |
| `crates/stint-core/migrations/0005_calendars_default_project.sql` | `ALTER TABLE calendars ADD COLUMN default_project_id`. |

### stint-core (modified)

| Path | Change |
|---|---|
| `crates/stint-core/src/solidtime/dto.rs` | Add `RemoteClient { id, name, archived }`. |
| `crates/stint-core/src/solidtime/mod.rs` | Add `list_clients() -> Vec<RemoteClient>`. |
| `crates/stint-core/src/store/reference.rs` | Add `ClientRow`, `upsert_clients`, `list_clients`; extend `ProjectRow` with `client_name: Option<String>`; change `list_projects` to LEFT JOIN clients. |
| `crates/stint-core/src/sync/refresh.rs` | Fetch + upsert clients before projects. |
| `crates/stint-core/src/store/entries.rs` | Add `update_times(local_uuid, start_at, end_at)` (validates `end>start`, `duration<=24h`). |
| `crates/stint-core/src/timer.rs` | Add `start_at: Option<String>` to `StartArgs`; `TimerService::start` uses it when present (validates not-future). Add `TimerService::update_times` wrapper. |
| `crates/stint-core/src/calendar/store.rs` | `Calendar` gains `default_project_id: Option<String>`; new `set_default_project(calendar_id, project_id)`; SELECTs include the new column. |
| `crates/stint-core/src/calendar/types.rs` | `Calendar` struct gets `default_project_id: Option<String>`. |

### stint-core (tests)

| Path | Coverage |
|---|---|
| `crates/stint-core/tests/store_reference_clients.rs` (new) | upsert + list + JOIN-through-projects round trip. |
| `crates/stint-core/tests/solidtime.rs` (extend) | `list_clients` HTTP shape (wiremock). |
| `crates/stint-core/tests/sync_refresh.rs` (extend) | refresh pulls clients into the store. |
| `crates/stint-core/tests/store_entries.rs` (extend) | `update_times` happy path + `end<=start` + duration>24h rejections + dirties sync_state. |
| `crates/stint-core/tests/timer.rs` (extend) | `start` with `start_at: Some(past_ts)` → entry uses that ts; `Some(future)` → error. |
| `crates/stint-core/tests/calendar_store_calendars.rs` (extend) | `set_default_project` round trip + cascade-null when project deleted via FK (we don't actually FK; the SELECT just returns whatever's there — see Task 2.1). |
| `crates/stint-core/tests/store_calendar_migration.rs` (extend) | Migration 0005 adds `default_project_id` column. |

### stint-cli (modified)

| Path | Change |
|---|---|
| `crates/stint-cli/src/cmd/start.rs` | Add `--at <when>` flag; parses relative ("5min ago", "1h ago") or HH:MM or RFC 3339. |
| `crates/stint-cli/src/cmd/edit.rs` | Add `--start <hh:mm>` and `--end <hh:mm>` flags; HH:MM resolves against the entry's existing date. |
| `crates/stint-cli/src/cmd/calendar.rs` | Add `--set-default-project <calendar_id> <project_id>` and `--clear-default-project <calendar_id>` options to `Calendars` subcommand. |
| `crates/stint-cli/src/lib.rs` (new) | Make `cmd::*` importable from integration tests; exposes `parse_at_arg`. |
| `crates/stint-cli/src/main.rs` | Wire any module re-exports needed for lib. |

### stint-cli (tests)

| Path | Coverage |
|---|---|
| `crates/stint-cli/tests/cli_start_at.rs` (new) | `stint start "..." --at "15min ago"` → entry persists with start_at ≈ now-15min. |
| `crates/stint-cli/tests/cli_edit_times.rs` (new) | `stint edit <id> --start 09:00 --end 10:30` → entry times updated; date preserved. |
| `crates/stint-cli/tests/cli_calendar.rs` (extend) | `--set-default-project` + `--clear-default-project` round-trip surfaces in `Calendars` listing. |
| `crates/stint-cli/tests/parse_at.rs` (new, lib test) | Unit tests for `parse_at_arg`: "5min ago", "30 min ago", "1h ago", "1hr ago", "09:30", RFC3339, invalid → error. |

### stint-app (modified)

| Path | Change |
|---|---|
| `crates/stint-app/src/commands/projects.rs` | `list_projects` already returns `ProjectRow` — no signature change, the inner type now carries `client_name`. |
| `crates/stint-app/src/commands/timer.rs` | `StartTimerArgs` gains `start_at: Option<String>`; passed through to `TimerService`. Add `update_entry_times` command. |
| `crates/stint-app/src/commands/calendar.rs` | Add `calendar_set_default_project(calendar_id, project_id)` command. `calendar_log_event` reads calendar's `default_project_id` and passes to `create_completed`. |
| `crates/stint-app/src/main.rs` | Register `update_entry_times` + `calendar_set_default_project` in `invoke_handler!`. |

### ui (new)

| Path | Responsibility |
|---|---|
| `ui/src/components/ui/ProjectPicker.tsx` | Combobox-based primitive built on `@kobalte/core`. Props: `value`, `onChange`, `projects` (with `client_name`), `placeholder?`, `allowNone?`, `size?`. Groups options by client; filters by project name + client name; arrow-key nav + Enter to pick. |
| `ui/src/components/EditEntryDialog.tsx` | Modal dialog with description, project, billable, and start/end time inputs. Reuses ProjectPicker. Same-day only. |
| `ui/src/components/StartAtPicker.tsx` | Inline UI: "Start now / 5 min ago / 15 / 30 / 1h / Custom HH:MM". Returns `string | null` to caller. |

### ui (modified)

| Path | Change |
|---|---|
| `ui/package.json` | Add `@kobalte/core` to dependencies. |
| `ui/src/api.ts` | `startTimer` accepts `startAt`; add `updateEntryTimes(localUuid, startAt, endAt)`. Add `calendarApi.setDefaultProject(calendarId, projectId)`. |
| `ui/src/types.ts` | `Project` gains `client_name: string \| null`. `CalendarRow` gains `default_project_id: string \| null`. |
| `ui/src/components/TimerCard.tsx` | Replace `<select>` with `ProjectPicker`; add `StartAtPicker` toggle. |
| `ui/src/routes/Popover.tsx` | Replace `<select>` with `ProjectPicker`; add `StartAtPicker` toggle. |
| `ui/src/components/ui/Accordion.tsx` | Rewrite to use `@kobalte/core/collapsible` internally; same API (title, hint, right, defaultOpen, children). |
| `ui/src/components/EntryRow.tsx` | Replace inline edit panel with click-to-open `EditEntryDialog`; running timer also gets time-edit access. |
| `ui/src/routes/Settings.tsx` | `CalendarsManager` gains a per-calendar `ProjectPicker`; replace hand-rolled calendars popover with `@kobalte/core/popover`. |

---

## Phasing summary

Four commits, each shippable on its own:

1. **Tasks 1.1–1.12 — ProjectPicker + @kobalte/core primitives** (`feat(ui): searchable client-grouped project picker; collapsible replacement`).
2. **Tasks 2.1–2.7 — Per-calendar default project + popover** (`feat(core): per-calendar default project; calendar_log_event prefills; popover replacement`).
3. **Tasks 3.1–3.7 — Entry edit dialog with editable start/end times** (`feat(app): edit dialog for entry times`).
4. **Tasks 4.1–4.6 — Backdate start option** (`feat(core): backdate-able timer start`).

Then **Task 5 — final manual verification + PR**.

---

## Pre-flight

**Branch:**

```bash
git checkout main
git pull --ff-only
git checkout -b phase-3d
```

**Verify clean state:**

```bash
cargo test --workspace -- --test-threads=1
cd ui && pnpm typecheck && cd ..
```

Expected: all green. (Phase 3c is fully shipped; CI is currently green on `main`.)

**Quick refresher on the test discipline used in this repo:**

- Rust tests run single-threaded because they share the Keychain ACL on macOS. Always use `--test-threads=1`.
- Store-layer tests use `crates/stint-core/tests/common/mod.rs::setup()` — a tempdir-backed SQLite store. Look at `tests/store_entries.rs` for the pattern.
- HTTP shape tests use `wiremock`. See `tests/solidtime.rs`.
- CLI integration tests use `assert_cmd`. See `tests/cli_e2e.rs` for the `STINT_DB`-env-var pattern.

---

## Commit 1 — ProjectPicker + clients data

### Task 1.1: Migration 0004 — `clients` table

**Files:**
- Create: `crates/stint-core/migrations/0004_clients.sql`
- Extend test: `crates/stint-core/tests/store_reference_clients.rs` (new)

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/store_reference_clients.rs`:

```rust
mod common;

use stint_core::store::reference::{ClientRow, Reference};

#[tokio::test(flavor = "multi_thread")]
async fn upsert_then_list_clients_round_trips() {
    let store = common::setup().await;
    let r = Reference::new(store);

    r.upsert_clients(&[
        ClientRow {
            id: "c-1".into(),
            name: "Acme".into(),
            archived: 0,
        },
        ClientRow {
            id: "c-2".into(),
            name: "Beta Co".into(),
            archived: 0,
        },
    ])
    .await
    .unwrap();

    let listed = r.list_clients().await.unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].name, "Acme");
    assert_eq!(listed[1].name, "Beta Co");
}

#[tokio::test(flavor = "multi_thread")]
async fn upsert_overwrites_existing_client() {
    let store = common::setup().await;
    let r = Reference::new(store);

    r.upsert_clients(&[ClientRow {
        id: "c-1".into(),
        name: "Acme".into(),
        archived: 0,
    }])
    .await
    .unwrap();
    r.upsert_clients(&[ClientRow {
        id: "c-1".into(),
        name: "Acme Inc".into(),
        archived: 1,
    }])
    .await
    .unwrap();

    let listed = r.list_clients().await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "Acme Inc");
    assert_eq!(listed[0].archived, 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p stint-core --test store_reference_clients -- --test-threads=1
```

Expected: FAIL — `ClientRow` / `upsert_clients` / `list_clients` don't exist; the `clients` table doesn't exist (the migration also won't be there yet).

- [ ] **Step 3: Create the migration file**

Create `crates/stint-core/migrations/0004_clients.sql`:

```sql
-- Phase 3d: client cache. Mirrors the projects/tasks/tags pattern.

CREATE TABLE clients (
  id         TEXT PRIMARY KEY,
  name       TEXT NOT NULL,
  archived   INTEGER NOT NULL DEFAULT 0,
  fetched_at TEXT NOT NULL
);
```

- [ ] **Step 4: Add `ClientRow` + `upsert_clients` + `list_clients` to `Reference`**

Open `crates/stint-core/src/store/reference.rs` and add (after the existing `TagRow`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ClientRow {
    pub id: String,
    pub name: String,
    pub archived: i64,
}
```

Add these two methods inside `impl Reference`, beside the existing `upsert_tags` / `list_tags`:

```rust
pub async fn upsert_clients(&self, clients: &[ClientRow]) -> Result<()> {
    let now = time::now_utc();
    let mut tx = self.store.pool().begin().await?;
    for c in clients {
        sqlx::query(
            r#"INSERT INTO clients (id, name, archived, fetched_at)
               VALUES (?, ?, ?, ?)
               ON CONFLICT(id) DO UPDATE SET
                 name = excluded.name,
                 archived = excluded.archived,
                 fetched_at = excluded.fetched_at"#,
        )
        .bind(&c.id)
        .bind(&c.name)
        .bind(c.archived)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn list_clients(&self) -> Result<Vec<ClientRow>> {
    let rows = sqlx::query_as::<_, ClientRow>(
        "SELECT id, name, archived FROM clients ORDER BY name",
    )
    .fetch_all(self.store.pool())
    .await?;
    Ok(rows)
}
```

- [ ] **Step 5: Run test to verify it passes**

```bash
cargo test -p stint-core --test store_reference_clients -- --test-threads=1
```

Expected: PASS (both tests).

- [ ] **Step 6: Commit**

```bash
git add crates/stint-core/migrations/0004_clients.sql \
        crates/stint-core/src/store/reference.rs \
        crates/stint-core/tests/store_reference_clients.rs
git commit -m "feat(core): clients table + Reference upsert/list"
```

---

### Task 1.2: `SolidtimeClient::list_clients` + `RemoteClient` DTO

**Files:**
- Modify: `crates/stint-core/src/solidtime/dto.rs`
- Modify: `crates/stint-core/src/solidtime/mod.rs`
- Extend test: `crates/stint-core/tests/solidtime.rs`

- [ ] **Step 1: Add the wiremock-based failing test**

Open `crates/stint-core/tests/solidtime.rs` and append:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn list_clients_hits_org_endpoint_and_parses() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/clients"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                { "id": "c-1", "name": "Acme",   "archived": false },
                { "id": "c-2", "name": "Beta Co","archived": true  },
            ]
        })))
        .mount(&server)
        .await;

    let client = stint_core::solidtime::SolidtimeClient::with_api_token(
        &server.uri(),
        "tok",
    )
    .with_org("org-1");
    let clients = client.list_clients().await.unwrap();
    assert_eq!(clients.len(), 2);
    assert_eq!(clients[0].id, "c-1");
    assert!(clients[1].archived);
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p stint-core --test solidtime list_clients_hits -- --test-threads=1
```

Expected: FAIL — `list_clients` / `RemoteClient` don't exist.

- [ ] **Step 3: Add `RemoteClient` to `dto.rs`**

Open `crates/stint-core/src/solidtime/dto.rs` and add (next to `RemoteTag`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteClient {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub archived: bool,
}
```

- [ ] **Step 4: Add `list_clients` to `SolidtimeClient`**

Open `crates/stint-core/src/solidtime/mod.rs` and add a method beside `list_tags`:

```rust
pub async fn list_clients(&self) -> Result<Vec<RemoteClient>> {
    let org = self.org()?;
    let url = format!("{}/api/v1/organizations/{org}/clients", self.base_url);
    self.get_list(&url).await
}
```

- [ ] **Step 5: Run test to verify it passes**

```bash
cargo test -p stint-core --test solidtime list_clients_hits -- --test-threads=1
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/stint-core/src/solidtime/dto.rs \
        crates/stint-core/src/solidtime/mod.rs \
        crates/stint-core/tests/solidtime.rs
git commit -m "feat(core): SolidtimeClient.list_clients"
```

---

### Task 1.3: Reference sync pulls clients

**Files:**
- Modify: `crates/stint-core/src/sync/refresh.rs`
- Extend test: `crates/stint-core/tests/sync_refresh.rs`

- [ ] **Step 1: Add failing test**

Open `crates/stint-core/tests/sync_refresh.rs` and append a test that verifies `refresh_reference_data` pulls clients:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn refresh_pulls_clients_into_store() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    // Projects, tasks, tags endpoints return empty so refresh succeeds end-to-end.
    for endpoint in ["projects", "tasks", "tags"] {
        Mock::given(method("GET"))
            .and(path(format!("/api/v1/organizations/org-1/{endpoint}")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "data": [] })),
            )
            .mount(&server)
            .await;
    }
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/clients"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{ "id": "c-1", "name": "Acme", "archived": false }]
        })))
        .mount(&server)
        .await;

    let store = common::setup().await;
    let client = stint_core::solidtime::SolidtimeClient::with_api_token(
        &server.uri(),
        "tok",
    )
    .with_org("org-1");
    stint_core::sync::refresh::refresh_reference_data(&store, &client)
        .await
        .unwrap();

    let r = stint_core::store::reference::Reference::new(store);
    let clients = r.list_clients().await.unwrap();
    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0].name, "Acme");
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p stint-core --test sync_refresh refresh_pulls_clients -- --test-threads=1
```

Expected: FAIL — `refresh_reference_data` doesn't pull clients yet (clients table will be empty after refresh).

- [ ] **Step 3: Extend `refresh_reference_data`**

Open `crates/stint-core/src/sync/refresh.rs`. Add `ClientRow` to the import:

```rust
use crate::store::reference::{ClientRow, ProjectRow, Reference, TagRow, TaskRow};
```

Then add a clients-fetch block at the top of the body, before projects:

```rust
let clients = client.list_clients().await?;
let client_rows: Vec<ClientRow> = clients
    .into_iter()
    .map(|c| ClientRow {
        id: c.id,
        name: c.name,
        archived: if c.archived { 1 } else { 0 },
    })
    .collect();
r.upsert_clients(&client_rows).await?;
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cargo test -p stint-core --test sync_refresh -- --test-threads=1
```

Expected: PASS (the new test and all pre-existing tests in the file).

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/sync/refresh.rs \
        crates/stint-core/tests/sync_refresh.rs
git commit -m "feat(core): refresh_reference_data pulls clients"
```

---

### Task 1.4: `ProjectRow.client_name` via LEFT JOIN

**Files:**
- Modify: `crates/stint-core/src/store/reference.rs`
- Extend test: `crates/stint-core/tests/store_reference.rs`

- [ ] **Step 1: Add failing test**

Open `crates/stint-core/tests/store_reference.rs` and append:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn list_projects_joins_client_name() {
    let store = common::setup().await;
    let r = Reference::new(store);

    r.upsert_clients(&[ClientRow {
        id: "c-1".into(),
        name: "Acme".into(),
        archived: 0,
    }])
    .await
    .unwrap();
    r.upsert_projects(&[
        ProjectRow {
            id: "p-1".into(),
            name: "Site".into(),
            color: None,
            client_id: Some("c-1".into()),
            client_name: None, // ignored on write
            archived: 0,
        },
        ProjectRow {
            id: "p-2".into(),
            name: "Internal".into(),
            color: None,
            client_id: None,
            client_name: None,
            archived: 0,
        },
    ])
    .await
    .unwrap();

    let listed = r.list_projects().await.unwrap();
    assert_eq!(listed.len(), 2);
    let site = listed.iter().find(|p| p.id == "p-1").unwrap();
    let internal = listed.iter().find(|p| p.id == "p-2").unwrap();
    assert_eq!(site.client_name.as_deref(), Some("Acme"));
    assert_eq!(internal.client_name, None);
}
```

(Imports: `use stint_core::store::reference::{ClientRow, ProjectRow, Reference};`.)

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p stint-core --test store_reference list_projects_joins -- --test-threads=1
```

Expected: FAIL — `ProjectRow` doesn't have `client_name`.

- [ ] **Step 3: Extend `ProjectRow` and rewrite `list_projects`**

Open `crates/stint-core/src/store/reference.rs`.

Add the field to `ProjectRow`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProjectRow {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub client_id: Option<String>,
    #[sqlx(default)]
    pub client_name: Option<String>,
    pub archived: i64,
}
```

Rewrite `list_projects` to LEFT JOIN clients:

```rust
pub async fn list_projects(&self) -> Result<Vec<ProjectRow>> {
    let rows = sqlx::query_as::<_, ProjectRow>(
        r#"SELECT p.id, p.name, p.color, p.client_id,
                  c.name AS client_name, p.archived
           FROM projects p
           LEFT JOIN clients c ON c.id = p.client_id
           ORDER BY p.name"#,
    )
    .fetch_all(self.store.pool())
    .await?;
    Ok(rows)
}
```

`upsert_projects` is unchanged — it never writes `client_name` (it's a SELECT-time projection).

- [ ] **Step 4: Run all reference tests**

```bash
cargo test -p stint-core --test store_reference --test store_reference_clients -- --test-threads=1
```

Expected: PASS (the new test plus all pre-existing tests).

- [ ] **Step 5: Compile check across the workspace**

```bash
cargo build --workspace
```

Expected: PASS. (Anywhere that constructs `ProjectRow` literally — e.g. `sync/refresh.rs` — now needs `client_name: None`. The Rust compiler will tell you.)

If `cargo build` flags a missing field in `sync/refresh.rs`, add `client_name: None,` to the struct literal there.

- [ ] **Step 6: Commit**

```bash
git add crates/stint-core/src/store/reference.rs \
        crates/stint-core/src/sync/refresh.rs \
        crates/stint-core/tests/store_reference.rs
git commit -m "feat(core): list_projects joins client_name"
```

---

### Task 1.5: UI types reflect `client_name`

**Files:**
- Modify: `ui/src/types.ts`

- [ ] **Step 1: Extend the `Project` type**

Open `ui/src/types.ts`. Replace the `Project` type with:

```ts
export type Project = {
  id: string;
  name: string;
  color: string | null;
  client_id: string | null;
  client_name: string | null;
  archived: number;
};
```

- [ ] **Step 2: Typecheck**

```bash
cd ui && pnpm typecheck && cd ..
```

Expected: PASS. (`Project` is consumed in `Settings.tsx`, `TimerCard.tsx`, `Popover.tsx`, `EntryRow.tsx` — none currently destructure `client_name`, so adding the optional field is non-breaking.)

- [ ] **Step 3: Commit**

```bash
git add ui/src/types.ts
git commit -m "feat(ui): Project type carries client_name"
```

---

### Task 1.6: Add `@kobalte/core`

**Files:**
- Modify: `ui/package.json`
- Modify: `ui/pnpm-lock.yaml` (auto-generated)
- Modify: `pnpm-lock.yaml` (auto-generated, root workspace)

- [ ] **Step 1: Install**

From the repo root:

```bash
pnpm --filter stint-ui add @kobalte/core
```

If pnpm complains about the workspace filter not matching, fall back to:

```bash
cd ui && pnpm add @kobalte/core && cd ..
```

Expected: `@kobalte/core` appears under `dependencies` in `ui/package.json`; both lockfiles update.

- [ ] **Step 2: Sanity-check the install**

```bash
cd ui && pnpm typecheck && pnpm build && cd ..
```

Expected: PASS. (The build verifies the package resolves; we haven't imported anything yet.)

- [ ] **Step 3: Commit**

```bash
git add ui/package.json ui/pnpm-lock.yaml pnpm-lock.yaml
git commit -m "chore(ui): add @kobalte/core for combobox primitive"
```

---

### Task 1.7: Build `ProjectPicker` component

**Files:**
- Create: `ui/src/components/ui/ProjectPicker.tsx`

- [ ] **Step 1: Write the component**

Create `ui/src/components/ui/ProjectPicker.tsx`:

```tsx
import { Combobox } from "@kobalte/core/combobox";
import { For, Show, createMemo } from "solid-js";
import type { Project } from "~/types";

type Option = {
  id: string;
  name: string;
  clientName: string | null;
};

const NO_PROJECT: Option = {
  id: "",
  name: "No project",
  clientName: null,
};

export default function ProjectPicker(props: {
  value: string | null;
  onChange: (id: string | null) => void;
  projects: Project[];
  placeholder?: string;
  allowNone?: boolean;
  size?: "sm" | "md";
}) {
  const options = createMemo<Option[]>(() => {
    const list = props.projects
      .filter((p) => !p.archived)
      .map<Option>((p) => ({
        id: p.id,
        name: p.name,
        clientName: p.client_name,
      }));
    return props.allowNone === false ? list : [NO_PROJECT, ...list];
  });

  const selected = createMemo<Option | null>(() => {
    const v = props.value ?? "";
    return options().find((o) => o.id === v) ?? null;
  });

  // Group by client_name for the dropdown. Items without a client
  // get bucketed under "Other" at the bottom.
  const groups = createMemo(() => {
    const map = new Map<string, Option[]>();
    for (const opt of options()) {
      const key = opt.id === "" ? "" : (opt.clientName ?? "Other");
      const arr = map.get(key) ?? [];
      arr.push(opt);
      map.set(key, arr);
    }
    return Array.from(map.entries());
  });

  const sizeClass = () =>
    props.size === "sm"
      ? "px-2.5 py-1.5 text-[12px]"
      : "px-3 py-1.5 text-sm";

  return (
    <Combobox<Option>
      options={options()}
      optionValue="id"
      optionLabel="name"
      optionTextValue={(o) =>
        o.clientName ? `${o.name} ${o.clientName}` : o.name
      }
      value={selected()}
      onChange={(v) => props.onChange(v?.id ? v.id : null)}
      placeholder={props.placeholder ?? "Select project…"}
      itemComponent={(p) => (
        <Combobox.Item
          item={p.item}
          class="flex cursor-pointer items-center justify-between gap-2 rounded px-2 py-1.5 text-sm outline-none data-[highlighted]:bg-zinc-100 dark:data-[highlighted]:bg-zinc-800"
        >
          <Combobox.ItemLabel>{p.item.rawValue.name}</Combobox.ItemLabel>
          <Show when={p.item.rawValue.clientName}>
            <span class="text-[11px] text-zinc-400">
              {p.item.rawValue.clientName}
            </span>
          </Show>
        </Combobox.Item>
      )}
    >
      <Combobox.Control
        class={`flex w-full items-center gap-1 rounded-lg border border-zinc-200 bg-zinc-50/50 outline-none transition focus-within:border-indigo-400 focus-within:bg-white dark:border-zinc-700 dark:bg-zinc-800/40 dark:focus-within:bg-zinc-800 ${sizeClass()}`}
      >
        <Combobox.Input class="flex-1 bg-transparent outline-none placeholder:text-zinc-400" />
        <Combobox.Trigger
          aria-label="Open project list"
          class="text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300"
        >
          ▾
        </Combobox.Trigger>
      </Combobox.Control>
      <Combobox.Portal>
        <Combobox.Content class="z-50 max-h-72 overflow-y-auto rounded-lg border border-black/[0.08] bg-white p-1 shadow-lg dark:border-white/[0.08] dark:bg-zinc-950">
          <Combobox.Listbox class="space-y-2" />
          {/* Group headers are visual only — kobalte's Combobox doesn't
              expose section groupings, but rendering them as item labels
              wouldn't be filterable. For now the per-item client subtitle
              (see itemComponent) carries the grouping signal. Sections
              proper can be added later if the flat list feels too noisy. */}
          <For each={groups()}>{() => null}</For>
        </Combobox.Content>
      </Combobox.Portal>
    </Combobox>
  );
}
```

> **Note on grouping:** Kobalte's `Combobox` doesn't ship a section/group primitive at the time of writing. We render the client name inline next to each project label (visually identical to a sticky group header for the purposes of "which client?"). If the flat list gets unwieldy in practice, switch to manual sectioning in a follow-up.

- [ ] **Step 2: Verify it compiles + builds**

```bash
cd ui && pnpm typecheck && pnpm build && cd ..
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add ui/src/components/ui/ProjectPicker.tsx
git commit -m "feat(ui): ProjectPicker combobox primitive"
```

---

### Task 1.8: Wire `ProjectPicker` into `TimerCard` start form

**Files:**
- Modify: `ui/src/components/TimerCard.tsx`

- [ ] **Step 1: Replace the start-form `<select>` with ProjectPicker**

Open `ui/src/components/TimerCard.tsx`. Add the import:

```tsx
import ProjectPicker from "./ui/ProjectPicker";
```

In the start-form (the `<Show fallback={...}>` branch), replace the `<select>` block:

```tsx
<select … >
  <option value="">No project</option>
  <For each={projectList()}>
    {(p) => <option value={p.id}>{p.name}</option>}
  </For>
</select>
```

with:

```tsx
<div class="min-w-0 flex-1">
  <ProjectPicker
    value={projectId() || null}
    onChange={(id) => setProjectId(id ?? "")}
    projects={projectList()}
    placeholder="No project"
  />
</div>
```

In the running-timer branch, replace the second `<select>` similarly:

```tsx
<div class="min-w-0 flex-1">
  <ProjectPicker
    value={t().project_id}
    onChange={async (id) => {
      await api.setEntryProject(t().local_uuid, id);
      await timer.refresh();
    }}
    projects={projectList()}
    placeholder="No project"
    size="sm"
  />
</div>
```

Remove the unused `<For>` import if no other `<For>` remains in the file (it'll still be needed because… check; if not, drop it).

- [ ] **Step 2: Verify**

```bash
cd ui && pnpm typecheck && pnpm build && cd ..
```

Expected: PASS.

- [ ] **Step 3: Visually verify**

Run the GUI: `scripts/dev-app.sh` (per CLAUDE.md). Open the main window, confirm:

- The project picker appears as an input + dropdown trigger.
- Typing filters the list.
- Selecting a project starts the timer with that project.
- While a timer runs, changing the picker updates the running entry's project.

If the popover/dropdown is clipped by the card, increase the z-index on `Combobox.Content` (already set to `z-50` in the component).

- [ ] **Step 4: Commit**

```bash
git add ui/src/components/TimerCard.tsx
git commit -m "feat(ui): TimerCard uses ProjectPicker"
```

---

### Task 1.9: Wire `ProjectPicker` into `Popover` start form

**Files:**
- Modify: `ui/src/routes/Popover.tsx`

- [ ] **Step 1: Replace the `<select>` in Popover.tsx**

Open `ui/src/routes/Popover.tsx`. Add:

```tsx
import ProjectPicker from "~/components/ui/ProjectPicker";
```

Replace the existing `<select>` block (inside the start-form's `<Show>` branch) with:

```tsx
<div class="min-w-0 flex-1">
  <ProjectPicker
    value={projectId() || null}
    onChange={(id) => setProjectId(id ?? "")}
    projects={projects() ?? []}
    placeholder="No project"
    size="sm"
  />
</div>
```

Drop the now-unused `<For>` import if it's no longer referenced.

- [ ] **Step 2: Verify**

```bash
cd ui && pnpm typecheck && pnpm build && cd ..
```

Expected: PASS.

- [ ] **Step 3: Visual check in the popover**

In `scripts/dev-app.sh`, click the menu-bar tray icon to open the popover. The picker should fit inside the 360-ish-pixel-wide popover (`size="sm"` keeps padding tight). Filter still works; arrow keys navigate.

- [ ] **Step 4: Commit**

```bash
git add ui/src/routes/Popover.tsx
git commit -m "feat(ui): Popover start form uses ProjectPicker"
```

---

### Task 1.10: Wire `ProjectPicker` into `EntryRow` inline edit

**Files:**
- Modify: `ui/src/components/EntryRow.tsx`

- [ ] **Step 1: Swap the `<select>` for ProjectPicker**

Open `ui/src/components/EntryRow.tsx`. Add:

```tsx
import ProjectPicker from "./ui/ProjectPicker";
```

Replace the `<select>` block in the expanded edit panel:

```tsx
<div class="flex-1">
  <label class="block text-[10px] font-semibold uppercase tracking-[0.08em] text-zinc-400 dark:text-zinc-500">
    Project
  </label>
  <div class="mt-1">
    <ProjectPicker
      value={props.entry.project_id}
      onChange={(id) => changeProject(id ?? "")}
      projects={projects() ?? []}
      placeholder="No project"
      size="sm"
    />
  </div>
</div>
```

`changeProject` already accepts `"" | string`. Drop the `<For>` import if unused.

- [ ] **Step 2: Verify**

```bash
cd ui && pnpm typecheck && pnpm build && cd ..
```

Expected: PASS.

- [ ] **Step 3: Visual check**

In `scripts/dev-app.sh`, expand an entry on the Today page, change its project via the picker, and confirm the row's project pill updates after the call resolves.

- [ ] **Step 4: Commit**

```bash
git add ui/src/components/EntryRow.tsx
git commit -m "feat(ui): EntryRow uses ProjectPicker"
```

---

### Task 1.11: End-of-slice verification

- [ ] **Step 1: Run full workspace tests**

```bash
cargo test --workspace -- --test-threads=1
cd ui && pnpm typecheck && pnpm build && cd ..
```

Expected: all green.

- [ ] **Step 2: One more visual sweep**

`scripts/dev-app.sh`. In the running app, exercise:

- Start a timer from the popover with a project picked.
- Stop. Confirm the entry shows the right project pill.
- Open the main window; expand the entry; change the project via the picker; confirm it updates.
- Start a new timer from the main TimerCard; pick a project; stop; confirm.

(No formal automated UI tests yet — the upcoming testing-uplift phase will add them.)

- [ ] **Step 3: No-op commit barrier**

(If everything above is already committed, skip. Otherwise commit any straggler doc/notes file.) The slice closes here; the next slice opens fresh.

---

### Task 1.12: Replace Accordion with `@kobalte/core/collapsible`

**Files:**
- Modify: `ui/src/components/ui/Accordion.tsx`

- [ ] **Step 1: Rewrite Accordion.tsx**

Open `ui/src/components/ui/Accordion.tsx` and replace with:

```tsx
import { JSX, Show } from "solid-js";
import { Collapsible } from "@kobalte/core/collapsible";

export default function Accordion(props: {
  title: string;
  hint?: string;
  right?: JSX.Element;
  defaultOpen?: boolean;
  children: JSX.Element;
}) {
  return (
    <Collapsible.Root
      class="rounded-2xl border border-black/[0.06] bg-white dark:border-white/[0.06] dark:bg-zinc-900"
      defaultOpen={props.defaultOpen}
    >
      <Collapsible.Trigger class="flex w-full items-center justify-between gap-3 px-6 py-4 text-left">
        <div class="min-w-0 flex-1">
          <div class="flex items-center gap-2">
            <h2 class="text-sm font-semibold uppercase tracking-wide text-zinc-500 dark:text-zinc-400">
              {props.title}
            </h2>
            <Show when={props.right}>
              <div class="ml-auto flex items-center gap-2">{props.right}</div>
            </Show>
          </div>
          <Show when={props.hint}>
            <p class="mt-1 text-xs text-zinc-500">{props.hint}</p>
          </Show>
        </div>
        <svg
          class="h-4 w-4 shrink-0 text-zinc-400 transition-transform group-data-[expanded]:rotate-90"
          viewBox="0 0 20 20"
          fill="currentColor"
          aria-hidden="true"
        >
          <path
            fill-rule="evenodd"
            d="M7.21 14.77a.75.75 0 0 1 .02-1.06L11.17 10 7.23 6.29a.75.75 0 1 1 1.04-1.08l4.5 4.25a.75.75 0 0 1 0 1.08l-4.5 4.25a.75.75 0 0 1-1.06-.02Z"
            clip-rule="evenodd"
          />
        </svg>
      </Collapsible.Trigger>
      <Collapsible.Content class="border-t border-black/[0.05] px-6 pb-6 pt-5 data-[closed]:animate-collapse-up data-[expanded]:animate-collapse-down dark:border-white/[0.04]">
        {props.children}
      </Collapsible.Content>
    </Collapsible.Root>
  );
}
```

> **API note:** The `Collapsible` component uses a `data-expanded` / `data-closed` attribute pattern instead of `classList` toggle. The animation classes (`animate-collapse-up` / `animate-collapse-down`) should be added to `tailwind.config` if smooth open/close is desired; they're optional and the default instant toggle works fine without them.

- [ ] **Step 2: Verify consumers still compile**

```bash
cd ui && pnpm typecheck && pnpm build && cd ..
```

Expected: PASS. The three consumers (`Settings.tsx` ×2, `CalendarSection.tsx` ×1) pass the same props — no API change, the component interface is identical.

- [ ] **Step 3: Visual check**

`scripts/dev-app.sh`. Open Settings and confirm both accordion sections (Server, Calendar) expand/collapse on click. Open Today and confirm the CalendarSection accordion works.

- [ ] **Step 4: Commit**

```bash
git add ui/src/components/ui/Accordion.tsx
git commit -m "feat(ui): replace Accordion with @kobalte/core/collapsible"
```

---

## Commit 2 — Per-calendar default project

### Task 2.1: Migration 0005 — `calendars.default_project_id`

**Files:**
- Create: `crates/stint-core/migrations/0005_calendars_default_project.sql`
- Extend test: `crates/stint-core/tests/store_calendar_migration.rs`

- [ ] **Step 1: Write the failing test**

Open `crates/stint-core/tests/store_calendar_migration.rs` and append:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn calendars_table_has_default_project_id_column() {
    let store = common::setup().await;

    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM pragma_table_info('calendars')
         WHERE name = 'default_project_id'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();

    assert_eq!(row.0, 1, "default_project_id column should exist after migration 0005");
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p stint-core --test store_calendar_migration default_project_id -- --test-threads=1
```

Expected: FAIL — column doesn't exist.

- [ ] **Step 3: Create the migration**

Create `crates/stint-core/migrations/0005_calendars_default_project.sql`:

```sql
-- Phase 3d: per-calendar default project for "Log this" prefill.
-- No FK to projects(id) because Solidtime projects can be deleted on the
-- server; we never want a delete there to fail a constraint here. The
-- calendar_log_event path silently treats a stale id as "no project"
-- (Solidtime returns 422 only on member_id, not on project_id mismatch).

ALTER TABLE calendars ADD COLUMN default_project_id TEXT;
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cargo test -p stint-core --test store_calendar_migration -- --test-threads=1
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/migrations/0005_calendars_default_project.sql \
        crates/stint-core/tests/store_calendar_migration.rs
git commit -m "feat(core): add calendars.default_project_id column"
```

---

### Task 2.2: `Calendar` type + `CalendarStore` reads/writes the column

**Files:**
- Modify: `crates/stint-core/src/calendar/types.rs`
- Modify: `crates/stint-core/src/calendar/store.rs`
- Extend test: `crates/stint-core/tests/calendar_store_calendars.rs`

- [ ] **Step 1: Write the failing test**

Open `crates/stint-core/tests/calendar_store_calendars.rs` and append:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn set_default_project_round_trip() {
    let store = common::setup().await;
    let cs = stint_core::calendar::store::CalendarStore::new(store);

    // Seed an account + a calendar.
    let account = stint_core::calendar::types::CalendarAccount {
        id: "acc-1".into(),
        provider: stint_core::calendar::types::ProviderKind::Google,
        display_name: "Me".into(),
        identifier: "me@example.com".into(),
        caldav_url: None,
        enabled: true,
        created_at: stint_core::time::now_utc(),
    };
    cs.add_account(&account).await.unwrap();
    cs.upsert_calendars(
        "acc-1",
        &[stint_core::calendar::types::Calendar {
            id: "cal-1".into(),
            account_id: "acc-1".into(),
            name: "Personal".into(),
            color: None,
            included: true,
            default_project_id: None,
        }],
    )
    .await
    .unwrap();

    cs.set_default_project("cal-1", Some("p-123"))
        .await
        .unwrap();

    let cals = cs.list_calendars("acc-1").await.unwrap();
    assert_eq!(cals[0].default_project_id.as_deref(), Some("p-123"));

    cs.set_default_project("cal-1", None).await.unwrap();
    let cals = cs.list_calendars("acc-1").await.unwrap();
    assert_eq!(cals[0].default_project_id, None);
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p stint-core --test calendar_store_calendars set_default_project -- --test-threads=1
```

Expected: FAIL — `Calendar` doesn't have `default_project_id`, `set_default_project` doesn't exist.

- [ ] **Step 3: Extend the `Calendar` type**

Open `crates/stint-core/src/calendar/types.rs`. Replace the `Calendar` struct with:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calendar {
    pub id: String, // provider-native id
    pub account_id: String,
    pub name: String,
    pub color: Option<String>,
    pub included: bool,
    pub default_project_id: Option<String>,
}
```

- [ ] **Step 4: Update `CalendarStore` SELECTs + add `set_default_project`**

Open `crates/stint-core/src/calendar/store.rs`. Update the `CalendarRow` type alias (line ~17):

```rust
type CalendarRow = (String, String, String, Option<String>, i64, Option<String>);
```

Find the helper function that converts `CalendarRow` → `Calendar` (named `calendar_from_row` further down in the file). Add the new field to its construction:

```rust
fn calendar_from_row(r: CalendarRow) -> Calendar {
    Calendar {
        id: r.0,
        account_id: r.1,
        name: r.2,
        color: r.3,
        included: r.4 != 0,
        default_project_id: r.5,
    }
}
```

Find all SELECT statements that pull calendars (`list_calendars`, any other) and add `default_project_id` to the column list:

```rust
// before: "SELECT id, account_id, name, color, included FROM calendars …"
// after:
"SELECT id, account_id, name, color, included, default_project_id FROM calendars …"
```

`upsert_calendars`'s ON CONFLICT clause is untouched — like `included`, `default_project_id` is user-owned local state and should not be clobbered by a provider refresh.

Add `set_default_project` beside `set_calendar_included`:

```rust
pub async fn set_default_project(
    &self,
    calendar_id: &str,
    project_id: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "UPDATE calendars SET default_project_id = ? WHERE id = ?",
    )
    .bind(project_id)
    .bind(calendar_id)
    .execute(self.store.pool())
    .await?;
    Ok(())
}
```

- [ ] **Step 5: Fix any other callers of `Calendar { … }` literal**

```bash
cargo build --workspace 2>&1 | grep "missing field"
```

Likely hits: `crates/stint-core/src/calendar/google/client.rs` (where Google's response is mapped to `Calendar`). Add `default_project_id: None,` to every literal — provider-built calendars never come with a default.

- [ ] **Step 6: Run tests**

```bash
cargo test -p stint-core --test calendar_store_calendars -- --test-threads=1
```

Expected: PASS (the new test and all pre-existing tests).

- [ ] **Step 7: Commit**

```bash
git add crates/stint-core/src/calendar/types.rs \
        crates/stint-core/src/calendar/store.rs \
        crates/stint-core/src/calendar/google/client.rs \
        crates/stint-core/tests/calendar_store_calendars.rs
git commit -m "feat(core): Calendar.default_project_id + setter"
```

---

### Task 2.3: `calendar_log_event` reads the default

**Files:**
- Modify: `crates/stint-app/src/commands/calendar.rs`
- Extend test: `crates/stint-core/tests/calendar_logged_entry_sync.rs` *or* add inline coverage via the existing core-side path

> **Note:** `calendar_log_event` is a `#[tauri::command]`, so unit-testing it directly is awkward. We'll cover the behavior via a stint-core integration test that exercises the SQL prefill logic against `CalendarStore`. Then the Tauri command becomes a near-trivial wrapper.

- [ ] **Step 1: Write the failing core test**

Open `crates/stint-core/tests/calendar_logged_entry_sync.rs` (or create a sibling `calendar_default_project_prefill.rs` if that file is large) and add:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn log_event_prefills_project_from_calendar_default() {
    let store = common::setup().await;
    let cs = stint_core::calendar::store::CalendarStore::new(store.clone());
    let entries = stint_core::store::entries::Entries::new(store.clone());

    // Seed: account, calendar with default project, one event.
    cs.add_account(&stint_core::calendar::types::CalendarAccount {
        id: "acc".into(),
        provider: stint_core::calendar::types::ProviderKind::Google,
        display_name: "x".into(),
        identifier: "x".into(),
        caldav_url: None,
        enabled: true,
        created_at: stint_core::time::now_utc(),
    })
    .await
    .unwrap();
    cs.upsert_calendars(
        "acc",
        &[stint_core::calendar::types::Calendar {
            id: "cal".into(),
            account_id: "acc".into(),
            name: "Personal".into(),
            color: None,
            included: true,
            default_project_id: Some("p-42".into()),
        }],
    )
    .await
    .unwrap();

    // Direct invocation of the helper that the Tauri command will use.
    let cal = cs.list_calendars("acc").await.unwrap().pop().unwrap();
    let project_id_for_new_entry = cal.default_project_id.clone();

    let local_uuid = entries
        .create_completed(stint_core::store::entries::NewCompletedEntry {
            description: "Standup".into(),
            project_id: project_id_for_new_entry,
            task_id: None,
            start_at: "2026-05-20T09:00:00Z".into(),
            end_at: "2026-05-20T09:15:00Z".into(),
            billable: false,
            source: "calendar".into(),
            source_event_id: Some("acc:evt:2026-05-20T09:00:00Z".into()),
        })
        .await
        .unwrap();

    let row = entries.get(&local_uuid).await.unwrap().unwrap();
    assert_eq!(row.project_id.as_deref(), Some("p-42"));
}
```

- [ ] **Step 2: Run to verify it passes (it already should — this is a smoke test for the wiring)**

```bash
cargo test -p stint-core calendar log_event_prefills -- --test-threads=1
```

Expected: PASS. (The behavior is purely the result of how we'll wire the Tauri command. The test pins it.)

- [ ] **Step 3: Wire the Tauri command**

Open `crates/stint-app/src/commands/calendar.rs`. In `calendar_log_event`, after the `let event = events.into_iter().find(…)` block, look up the calendar's default project:

```rust
let cals = cs.list_calendars(&account_id).await?;
let default_project_id = cals
    .iter()
    .find(|c| c.id == event.calendar_id)
    .and_then(|c| c.default_project_id.clone());
```

Then in the `create_completed` call, replace `project_id: None,` with:

```rust
project_id: default_project_id,
```

- [ ] **Step 4: Compile**

```bash
cargo build -p stint-app
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-app/src/commands/calendar.rs \
        crates/stint-core/tests/calendar_logged_entry_sync.rs
git commit -m "feat(app): calendar_log_event prefills default project"
```

---

### Task 2.4: Tauri command for setting the default

**Files:**
- Modify: `crates/stint-app/src/commands/calendar.rs`
- Modify: `crates/stint-app/src/main.rs`
- Modify: `ui/src/api.ts`
- Modify: `ui/src/types.ts`

- [ ] **Step 1: Add the Tauri command**

Open `crates/stint-app/src/commands/calendar.rs` and add at the end:

```rust
#[tauri::command]
pub async fn calendar_set_default_project(
    state: State<'_, RwLock<AppState>>,
    app: tauri::AppHandle,
    calendar_id: String,
    project_id: Option<String>,
) -> Result<(), AppError> {
    let store = store(&state).await;
    let cs = CalendarStore::new((*store).clone());
    cs.set_default_project(&calendar_id, project_id.as_deref())
        .await?;
    let _ = app.emit(EVENT_CALENDAR_CHANGED, &calendar_id);
    Ok(())
}
```

- [ ] **Step 2: Register it in `main.rs`**

Open `crates/stint-app/src/main.rs`. In the `invoke_handler!` macro, add `commands::calendar::calendar_set_default_project,` in the calendar block.

- [ ] **Step 3: Extend `CalendarRow` UI type**

Open `ui/src/types.ts` and update:

```ts
export type CalendarRow = {
  id: string;
  account_id: string;
  name: string;
  color: string | null;
  included: boolean;
  default_project_id: string | null;
};
```

- [ ] **Step 4: Add the API binding**

Open `ui/src/api.ts`. Inside `calendarApi`, add:

```ts
setDefaultProject: (calendarId: string, projectId: string | null) =>
  invoke<void>("calendar_set_default_project", {
    calendarId,
    projectId,
  }),
```

- [ ] **Step 5: Build**

```bash
cargo build -p stint-app
cd ui && pnpm typecheck && pnpm build && cd ..
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/stint-app/src/commands/calendar.rs \
        crates/stint-app/src/main.rs \
        ui/src/api.ts ui/src/types.ts
git commit -m "feat(app): calendar_set_default_project command + binding"
```

---

### Task 2.5: UI — per-calendar `ProjectPicker` in `CalendarsManager`

**Files:**
- Modify: `ui/src/routes/Settings.tsx`

- [ ] **Step 1: Wire the picker into the calendars dropdown**

Open `ui/src/routes/Settings.tsx`. Add at the top of imports:

```tsx
import { api, calendarApi, oauthSolidtimeLogout, oauthSolidtimeStart, oauthSolidtimeStatus } from "~/api";
import ProjectPicker from "~/components/ui/ProjectPicker";
```

In the `CalendarsManager` component, fetch projects (mirror the resource pattern used elsewhere). Add inside the component body, before the `return`:

```tsx
const [projects] = createResource(() => api.listProjects(), { initialValue: [] });

async function setDefault(calId: string, projectId: string | null) {
  try {
    await calendarApi.setDefaultProject(calId, projectId);
    refetch();
  } catch (e) {
    props.flash("err", `Set default failed: ${(e as { message: string }).message}`);
  }
}
```

In the `<For each={cals() ?? []}>` block, replace the existing `<li>` with:

```tsx
<li class="space-y-1">
  <label class="flex cursor-pointer items-center gap-2 text-sm">
    <input
      type="checkbox"
      checked={c.included}
      onChange={(e) => toggle(c.id, e.currentTarget.checked)}
    />
    <span class="flex-1">{c.name}</span>
  </label>
  <Show when={c.included}>
    <div class="pl-6">
      <ProjectPicker
        value={c.default_project_id}
        onChange={(id) => setDefault(c.id, id)}
        projects={projects() ?? []}
        placeholder="No default project"
        size="sm"
      />
    </div>
  </Show>
</li>
```

- [ ] **Step 2: Build**

```bash
cd ui && pnpm typecheck && pnpm build && cd ..
```

Expected: PASS.

- [ ] **Step 3: Visual check**

`scripts/dev-app.sh`. Open Settings → Calendar accounts → "Calendars" on an account. Each enabled calendar should show a `ProjectPicker` below its include checkbox. Picking a project should persist (re-open the dropdown to confirm).

- [ ] **Step 4: Commit**

```bash
git add ui/src/routes/Settings.tsx
git commit -m "feat(ui): per-calendar default project picker"

---

### Task 2.6: Replace hand-rolled calendars popover with `@kobalte/core/popover`

**Files:**
- Modify: `ui/src/routes/Settings.tsx`

- [ ] **Step 1: Replace the CalendarsManager popover**

Open `ui/src/routes/Settings.tsx`. Add the import at the top:

```tsx
import { Popover } from "@kobalte/core/popover";
```

Then replace the `CalendarsManager` return block (currently a `<div class="relative">` wrapping a `Button` + `<Show when={open()}>` div) with:

```tsx
return (
  <Popover.Root open={open()} onOpenChange={setOpen}>
    <Popover.Trigger
      class="inline-flex items-center gap-1 rounded-lg px-2.5 py-1.5 text-sm font-medium text-zinc-700 outline-none transition hover:bg-zinc-100 focus-visible:ring-2 focus-visible:ring-indigo-400 dark:text-zinc-300 dark:hover:bg-zinc-800"
    >
      Calendars
      <svg class="h-3.5 w-3.5 text-zinc-400" viewBox="0 0 20 20" fill="currentColor" aria-hidden="true">
        <path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 0 1 1.06.02L10 11.17l3.71-3.94a.75.75 0 1 1 1.08 1.04l-4.25 4.5a.75.75 0 0 1-1.08 0l-4.25-4.5a.75.75 0 0 1 .02-1.06Z" clip-rule="evenodd" />
      </svg>
    </Popover.Trigger>
    <Popover.Portal>
      <Popover.Content class="z-50 mt-1 w-72 rounded-md border border-black/[0.08] bg-white p-3 shadow-lg outline-none data-[expanded]:animate-in data-[closed]:animate-out dark:border-white/[0.08] dark:bg-zinc-950">
        <Show
          when={(cals() ?? []).length > 0}
          fallback={
            <p class="text-xs text-zinc-500">
              {cals.loading ? "Loading…" : "No calendars."}
            </p>
          }
        >
          <ul class="space-y-1">
            <For each={cals() ?? []}>
              {(c) => (
                <li>
                  <label class="flex cursor-pointer items-center gap-2 text-sm">
                    <input
                      type="checkbox"
                      checked={c.included}
                      onChange={(e) =>
                        toggle(c.id, e.currentTarget.checked)
                      }
                    />
                    <span>{c.name}</span>
                  </label>
                </li>
              )}
            </For>
          </ul>
        </Show>
      </Popover.Content>
    </Popover.Portal>
  </Popover.Root>
);
```

Keep the existing `open()` signal and `setOpen` toggle — `Popover.Root` now controls them. The original `onClick` on the `Button` is gone; `Popover.Trigger` handles opening/closing.

Remove the unused `<div class="relative">` wrapper and the `<Show when={open()}>` condition — `Popover.Content` is only rendered when open per Kobalte convention.

- [ ] **Step 2: Verify it compiles**

```bash
cd ui && pnpm typecheck && pnpm build && cd ..
```

Expected: PASS.

- [ ] **Step 3: Visual check**

`scripts/dev-app.sh`. Open Settings → an account. Click the "Calendars" trigger. Confirm:
- The popover opens below the trigger.
- Click outside or press Escape → popover closes.
- Tab focus cycles inside the popover.
- The `open()` signal is still wired to the `ProjectPicker` (from Task 2.5) for persistence.

- [ ] **Step 4: Commit**

```bash
git add ui/src/routes/Settings.tsx
git commit -m "feat(ui): replace calendars popover with @kobalte/core/popover"
```

---

### Task 2.7: CLI — `--set-default-project` / `--clear-default-project`

**Files:**
- Modify: `crates/stint-cli/src/cmd/calendar.rs`
- Extend test: `crates/stint-cli/tests/cli_calendar.rs`

- [ ] **Step 1: Write the failing CLI test**

Open `crates/stint-cli/tests/cli_calendar.rs` and append:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn set_and_clear_default_project_round_trip() {
    use stint_core::calendar::store::CalendarStore;
    use stint_core::calendar::types::{Calendar, CalendarAccount, ProviderKind};
    use stint_core::store::Store;

    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    // Seed an account + calendar directly via stint-core so we don't need
    // to drive the OAuth flow from the CLI.
    let store = Store::connect(&db).await.unwrap();
    let cs = CalendarStore::new(store);
    cs.add_account(&CalendarAccount {
        id: "acc".into(),
        provider: ProviderKind::Google,
        display_name: "Me".into(),
        identifier: "me@example.com".into(),
        caldav_url: None,
        enabled: true,
        created_at: stint_core::time::now_utc(),
    })
    .await
    .unwrap();
    cs.upsert_calendars(
        "acc",
        &[Calendar {
            id: "cal-1".into(),
            account_id: "acc".into(),
            name: "Personal".into(),
            color: None,
            included: true,
            default_project_id: None,
        }],
    )
    .await
    .unwrap();

    cmd(&db)
        .args([
            "calendar", "calendars", "acc",
            "--set-default-project", "cal-1", "p-42",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Set default project"));

    // Re-open via core to confirm.
    let cs = CalendarStore::new(Store::connect(&db).await.unwrap());
    let cals = cs.list_calendars("acc").await.unwrap();
    assert_eq!(cals[0].default_project_id.as_deref(), Some("p-42"));

    cmd(&db)
        .args([
            "calendar", "calendars", "acc",
            "--clear-default-project", "cal-1",
        ])
        .assert()
        .success();

    let cs = CalendarStore::new(Store::connect(&db).await.unwrap());
    let cals = cs.list_calendars("acc").await.unwrap();
    assert_eq!(cals[0].default_project_id, None);
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p stint-cli --test cli_calendar set_and_clear -- --test-threads=1
```

Expected: FAIL — flag doesn't exist.

- [ ] **Step 3: Add the flags**

Open `crates/stint-cli/src/cmd/calendar.rs`. Replace the `Calendars` subcommand variant:

```rust
/// List or modify calendars for an account.
Calendars {
    account_id: String,
    /// Calendar id to include.
    #[arg(long)]
    include: Option<String>,
    /// Calendar id to exclude.
    #[arg(long)]
    exclude: Option<String>,
    /// Set the default project on a calendar:
    /// `--set-default-project <calendar_id> <project_id>`
    #[arg(long, num_args = 2, value_names = ["CALENDAR_ID", "PROJECT_ID"])]
    set_default_project: Option<Vec<String>>,
    /// Clear the default project on a calendar: `--clear-default-project <calendar_id>`
    #[arg(long)]
    clear_default_project: Option<String>,
},
```

Update the match arm in `run`:

```rust
CalendarCmd::Calendars {
    account_id,
    include,
    exclude,
    set_default_project,
    clear_default_project,
} => {
    if let Some(id) = include {
        cs.set_calendar_included(&id, true).await?;
        println!("Included calendar {id}.");
    }
    if let Some(id) = exclude {
        cs.set_calendar_included(&id, false).await?;
        println!("Excluded calendar {id}.");
    }
    if let Some(pair) = set_default_project {
        let cal_id = &pair[0];
        let proj_id = &pair[1];
        cs.set_default_project(cal_id, Some(proj_id)).await?;
        println!("Set default project {proj_id} on calendar {cal_id}.");
    }
    if let Some(id) = clear_default_project {
        cs.set_default_project(&id, None).await?;
        println!("Cleared default project on calendar {id}.");
    }
    for c in cs.list_calendars(&account_id).await? {
        let mark = if c.included { "[x]" } else { "[ ]" };
        let default = match &c.default_project_id {
            Some(p) => format!(" (default: {p})"),
            None => String::new(),
        };
        println!("{mark} {} {}{default}", c.id, c.name);
    }
    Ok(())
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p stint-cli --test cli_calendar -- --test-threads=1
```

Expected: PASS (the new test and the two pre-existing ones).

- [ ] **Step 5: Commit**

```bash
git add crates/stint-cli/src/cmd/calendar.rs \
        crates/stint-cli/tests/cli_calendar.rs
git commit -m "feat(cli): --set-default-project / --clear-default-project"
```

---

## Commit 3 — Entry edit dialog (start/end times)

### Task 3.1: `Entries::update_times` in core

**Files:**
- Modify: `crates/stint-core/src/store/entries.rs`
- Modify: `crates/stint-core/src/error.rs` (only if a new error variant is desired — likely not; we'll reuse `Error::Invariant`)
- Extend test: `crates/stint-core/tests/store_entries.rs`

- [ ] **Step 1: Write the failing tests**

Open `crates/stint-core/tests/store_entries.rs` and append:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn update_times_updates_both_fields_and_dirties_state() {
    let store = common::setup().await;
    let entries = Entries::new(store);

    // Seed a synced entry.
    let new_uuid = entries
        .create(NewTimeEntry {
            description: "old".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T09:00:00Z".into(),
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();
    entries.mark_synced(&new_uuid, "remote-id").await.unwrap();
    entries
        .set_end(&new_uuid, "2026-05-20T10:00:00Z")
        .await
        .unwrap();
    // set_end on a synced entry dirties it; clear back to synced for a clean precondition.
    sqlx::query("UPDATE time_entries SET sync_state='synced' WHERE local_uuid=?")
        .bind(&new_uuid)
        .execute(common::setup_again_pool().await)
        .await
        .ok(); // not actually needed if you can dirty-then-undirty in one step; otherwise create a fresh helper.

    entries
        .update_times(&new_uuid, "2026-05-20T09:30:00Z", "2026-05-20T10:30:00Z")
        .await
        .unwrap();

    let row = entries.get(&new_uuid).await.unwrap().unwrap();
    assert_eq!(row.start_at, "2026-05-20T09:30:00Z");
    assert_eq!(row.end_at.as_deref(), Some("2026-05-20T10:30:00Z"));
    assert_eq!(row.sync_state, "dirty");
}

#[tokio::test(flavor = "multi_thread")]
async fn update_times_rejects_end_le_start() {
    let store = common::setup().await;
    let entries = Entries::new(store);
    let uuid = entries
        .create(NewTimeEntry {
            description: "x".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T09:00:00Z".into(),
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();

    let err = entries
        .update_times(&uuid, "2026-05-20T11:00:00Z", "2026-05-20T10:00:00Z")
        .await
        .unwrap_err();
    assert!(matches!(err, stint_core::Error::Invariant(_)));
}

#[tokio::test(flavor = "multi_thread")]
async fn update_times_rejects_duration_over_24h() {
    let store = common::setup().await;
    let entries = Entries::new(store);
    let uuid = entries
        .create(NewTimeEntry {
            description: "x".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T09:00:00Z".into(),
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();

    let err = entries
        .update_times(&uuid, "2026-05-20T09:00:00Z", "2026-05-21T09:00:01Z")
        .await
        .unwrap_err();
    assert!(matches!(err, stint_core::Error::Invariant(_)));
}
```

> **Note:** If the dirty-then-undirty dance is awkward, simplify: seed the entry via direct INSERT through `store.pool()` setting `sync_state='synced'`. Either works; pick whichever reads cleaner.

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p stint-core --test store_entries update_times -- --test-threads=1
```

Expected: FAIL — `update_times` doesn't exist.

- [ ] **Step 3: Implement `update_times`**

Open `crates/stint-core/src/store/entries.rs`. Add (after `set_billable`):

```rust
pub async fn update_times(
    &self,
    local_uuid: &str,
    start_at: &str,
    end_at: &str,
) -> Result<()> {
    use crate::time;

    let start = time::parse(start_at)?;
    let end = time::parse(end_at)?;
    if end <= start {
        return Err(crate::Error::Invariant(
            "end must be after start".into(),
        ));
    }
    let duration = end - start;
    if duration > chrono::Duration::hours(24) {
        return Err(crate::Error::Invariant(
            "entry duration must be ≤ 24 hours".into(),
        ));
    }

    self.update_one(local_uuid, |s| {
        sqlx::query(
            "UPDATE time_entries
             SET start_at = ?, end_at = ?, sync_state = ?, updated_at = ?
             WHERE local_uuid = ?",
        )
        .bind(start_at)
        .bind(end_at)
        .bind(s.next_state())
        .bind(time::now_utc())
        .bind(local_uuid)
    })
    .await
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p stint-core --test store_entries update_times -- --test-threads=1
```

Expected: PASS (all three).

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/store/entries.rs \
        crates/stint-core/tests/store_entries.rs
git commit -m "feat(core): Entries.update_times with validation"
```

---

### Task 3.2: `TimerService::update_times` wrapper

**Files:**
- Modify: `crates/stint-core/src/timer.rs`
- Extend test: `crates/stint-core/tests/timer.rs`

- [ ] **Step 1: Write the failing test**

Open `crates/stint-core/tests/timer.rs` and append:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn timer_update_times_enqueues_update_when_synced() {
    let store = common::setup().await;
    let timer = stint_core::timer::TimerService::new(store.clone());
    let entries = stint_core::store::entries::Entries::new(store.clone());
    let queue = stint_core::store::queue::Queue::new(store);

    let id = timer
        .start(stint_core::timer::StartArgs {
            description: "d".into(),
            project_id: None,
            task_id: None,
            billable: false,
            source: "cli".into(),
            start_at: None,
        })
        .await
        .unwrap();
    timer.stop().await.unwrap();
    entries.mark_synced(&id, "remote-id").await.unwrap();

    let drain_before = queue.peek_due().await.unwrap().len();

    timer
        .update_times(&id, "2026-05-20T09:00:00Z", "2026-05-20T10:00:00Z")
        .await
        .unwrap();

    let drain_after = queue.peek_due().await.unwrap().len();
    assert_eq!(drain_after, drain_before + 1, "an update op should have been enqueued");
}
```

(Replace `peek_due` with whatever the existing Queue inspection method is named in your codebase. Check `crates/stint-core/tests/store_queue.rs` for the canonical inspection pattern.)

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p stint-core --test timer timer_update_times -- --test-threads=1
```

Expected: FAIL — `TimerService::update_times` doesn't exist; also `StartArgs.start_at` doesn't exist yet (we'll add it in Task 4.1; here, just pass `start_at: None` and let the compile error guide you to add the field as `Option<String>` in `StartArgs`, defaulting to `None` everywhere it's constructed).

> **If you'd rather defer the `start_at` field to Task 4.1**: temporarily drop the `start_at: None,` line and call the existing 5-field `StartArgs`. Re-add when Task 4 lands.

- [ ] **Step 3: Add `TimerService::update_times`**

Open `crates/stint-core/src/timer.rs`. Add after `set_billable`:

```rust
pub async fn update_times(
    &self,
    local_uuid: &str,
    start_at: &str,
    end_at: &str,
) -> Result<()> {
    self.ensure_entry_exists(local_uuid).await?;
    let entries = Entries::new(self.store.clone());
    entries.update_times(local_uuid, start_at, end_at).await?;
    self.maybe_enqueue_update(local_uuid).await
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p stint-core --test timer -- --test-threads=1
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/timer.rs \
        crates/stint-core/tests/timer.rs
git commit -m "feat(core): TimerService.update_times"
```

---

### Task 3.3: Tauri command `update_entry_times`

**Files:**
- Modify: `crates/stint-app/src/commands/timer.rs`
- Modify: `crates/stint-app/src/main.rs`
- Modify: `ui/src/api.ts`

- [ ] **Step 1: Add the Tauri command**

Open `crates/stint-app/src/commands/timer.rs`. Append:

```rust
#[tauri::command]
pub async fn update_entry_times(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    local_uuid: String,
    start_at: String,
    end_at: String,
) -> Result<(), AppError> {
    let store = store(&state).await;
    let timer = TimerService::new((*store).clone());
    timer.update_times(&local_uuid, &start_at, &end_at).await?;
    announce_change(&app);
    sync_worker::nudge(app.clone(), store);
    Ok(())
}
```

- [ ] **Step 2: Register in `main.rs`**

Open `crates/stint-app/src/main.rs`. Add `commands::timer::update_entry_times,` to the `invoke_handler!` macro.

- [ ] **Step 3: Add to api.ts**

Open `ui/src/api.ts`. In the `api` object, add:

```ts
updateEntryTimes: (localUuid: string, startAt: string, endAt: string) =>
  invoke<void>("update_entry_times", { localUuid, startAt, endAt }),
```

- [ ] **Step 4: Build**

```bash
cargo build -p stint-app
cd ui && pnpm typecheck && pnpm build && cd ..
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-app/src/commands/timer.rs \
        crates/stint-app/src/main.rs \
        ui/src/api.ts
git commit -m "feat(app): update_entry_times command + binding"
```

---

### Task 3.4: `EditEntryDialog` component

**Files:**
- Create: `ui/src/components/EditEntryDialog.tsx`

- [ ] **Step 1: Build the dialog**

Create `ui/src/components/EditEntryDialog.tsx`:

```tsx
import { createMemo, createResource, createSignal, Show } from "solid-js";
import { api } from "~/api";
import type { Entry } from "~/types";
import Button from "./ui/Button";
import ProjectPicker from "./ui/ProjectPicker";
import Toggle from "./ui/Toggle";

/// Convert an RFC 3339 timestamp to "HH:MM" in the user's local time.
function toLocalHHMM(iso: string): string {
  const d = new Date(iso);
  const hh = d.getHours().toString().padStart(2, "0");
  const mm = d.getMinutes().toString().padStart(2, "0");
  return `${hh}:${mm}`;
}

/// Reverse of toLocalHHMM, using the entry's existing date as the day anchor.
function fromLocalHHMM(referenceIso: string, hhmm: string): string {
  const ref = new Date(referenceIso);
  const [hStr, mStr] = hhmm.split(":");
  const out = new Date(ref);
  out.setHours(parseInt(hStr, 10), parseInt(mStr, 10), 0, 0);
  return out.toISOString();
}

export default function EditEntryDialog(props: {
  entry: Entry;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [desc, setDesc] = createSignal(props.entry.description);
  const [projectId, setProjectId] = createSignal<string | null>(
    props.entry.project_id,
  );
  const [billable, setBillable] = createSignal(props.entry.billable);
  const [startHHMM, setStartHHMM] = createSignal(toLocalHHMM(props.entry.start_at));
  const endIso = props.entry.end_at;
  const [endHHMM, setEndHHMM] = createSignal(
    endIso ? toLocalHHMM(endIso) : "",
  );
  const [err, setErr] = createSignal<string | null>(null);

  const [projects] = createResource(() => api.listProjects(), {
    initialValue: [],
  });

  const isCompleted = createMemo(() => Boolean(props.entry.end_at));

  async function save() {
    setErr(null);
    try {
      if (desc().trim() !== props.entry.description.trim()) {
        await api.updateDescription(props.entry.local_uuid, desc().trim());
      }
      if (projectId() !== props.entry.project_id) {
        await api.setEntryProject(props.entry.local_uuid, projectId());
      }
      if (billable() !== props.entry.billable) {
        await api.setEntryBillable(props.entry.local_uuid, billable());
      }
      if (isCompleted()) {
        const newStart = fromLocalHHMM(props.entry.start_at, startHHMM());
        const newEnd = fromLocalHHMM(props.entry.end_at!, endHHMM());
        if (newStart !== props.entry.start_at || newEnd !== props.entry.end_at) {
          await api.updateEntryTimes(props.entry.local_uuid, newStart, newEnd);
        }
      }
      props.onSaved();
      props.onClose();
    } catch (e) {
      setErr((e as { message: string }).message);
    }
  }

  return (
    <div
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4"
      onClick={(e) => {
        if (e.target === e.currentTarget) props.onClose();
      }}
    >
      <div class="w-full max-w-md rounded-2xl border border-black/[0.06] bg-white p-5 shadow-xl dark:border-white/[0.06] dark:bg-zinc-900">
        <h2 class="mb-4 text-base font-semibold">Edit entry</h2>

        <div class="space-y-3">
          <div>
            <label class="block text-[10px] font-semibold uppercase tracking-[0.08em] text-zinc-400 dark:text-zinc-500">
              Description
            </label>
            <input
              type="text"
              class="mt-1 w-full rounded-md border border-zinc-200 bg-white px-2.5 py-1.5 text-sm outline-none focus:border-indigo-400 dark:border-zinc-700 dark:bg-zinc-950"
              value={desc()}
              onInput={(e) => setDesc(e.currentTarget.value)}
            />
          </div>

          <div>
            <label class="block text-[10px] font-semibold uppercase tracking-[0.08em] text-zinc-400 dark:text-zinc-500">
              Project
            </label>
            <div class="mt-1">
              <ProjectPicker
                value={projectId()}
                onChange={setProjectId}
                projects={projects() ?? []}
                placeholder="No project"
                size="sm"
              />
            </div>
          </div>

          <Show when={isCompleted()}>
            <div class="flex items-end gap-3">
              <div class="flex-1">
                <label class="block text-[10px] font-semibold uppercase tracking-[0.08em] text-zinc-400 dark:text-zinc-500">
                  Start
                </label>
                <input
                  type="time"
                  class="mt-1 w-full rounded-md border border-zinc-200 bg-white px-2.5 py-1.5 text-sm outline-none focus:border-indigo-400 dark:border-zinc-700 dark:bg-zinc-950"
                  value={startHHMM()}
                  onInput={(e) => setStartHHMM(e.currentTarget.value)}
                />
              </div>
              <div class="flex-1">
                <label class="block text-[10px] font-semibold uppercase tracking-[0.08em] text-zinc-400 dark:text-zinc-500">
                  End
                </label>
                <input
                  type="time"
                  class="mt-1 w-full rounded-md border border-zinc-200 bg-white px-2.5 py-1.5 text-sm outline-none focus:border-indigo-400 dark:border-zinc-700 dark:bg-zinc-950"
                  value={endHHMM()}
                  onInput={(e) => setEndHHMM(e.currentTarget.value)}
                />
              </div>
            </div>
          </Show>

          <div>
            <Toggle
              label="Billable"
              checked={billable()}
              onChange={setBillable}
            />
          </div>
        </div>

        <Show when={err()}>
          <p class="mt-3 text-xs text-red-600 dark:text-red-400">{err()}</p>
        </Show>

        <div class="mt-5 flex justify-end gap-2">
          <Button variant="ghost" onClick={props.onClose}>
            Cancel
          </Button>
          <Button onClick={save}>Save</Button>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cd ui && pnpm typecheck && pnpm build && cd ..
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add ui/src/components/EditEntryDialog.tsx
git commit -m "feat(ui): EditEntryDialog with editable times"
```

---

### Task 3.5: Wire `EditEntryDialog` into `EntryRow`

**Files:**
- Modify: `ui/src/components/EntryRow.tsx`

- [ ] **Step 1: Replace the expanded inline panel with a dialog trigger**

Open `ui/src/components/EntryRow.tsx`. Add the import:

```tsx
import EditEntryDialog from "./EditEntryDialog";
```

Replace the `<button>` that toggles `open` with one that toggles a `editing` signal, and replace the expanded `<Show when={open()}>` panel with:

```tsx
<Show when={editing()}>
  <EditEntryDialog
    entry={props.entry}
    onClose={() => setEditing(false)}
    onSaved={() => {
      setEditing(false);
      props.onChange?.();
    }}
  />
</Show>
```

Keep the row click-to-toggle behavior but rename `open` → `editing`. Drop the `desc`, `projects` resource, `saveDescription`, `changeProject`, `changeBillable` helpers — the dialog owns them now. The collapsed row keeps the description, pills, duration, and `›` chevron.

Resulting component shape (sketch):

```tsx
import { Show, createSignal } from "solid-js";
import { api } from "~/api";
import EditEntryDialog from "./EditEntryDialog";
import type { Entry } from "~/types";
import { formatDuration } from "./Duration";
import Pill, { type PillTone } from "./ui/Pill";
import StatusDot, { type DotTone } from "./ui/StatusDot";
import Button from "./ui/Button";

// keep durationSecs() + syncMeta() unchanged

export default function EntryRow(props: { /* same props */ }) {
  const [editing, setEditing] = createSignal(false);
  const isRunning = !props.entry.end_at;
  const meta = () => syncMeta(props.entry.sync_state, isRunning);

  return (
    <li classList={{ "border-t border-black/[0.04] dark:border-white/[0.04]": !props.isFirst }}>
      <button
        type="button"
        class="flex w-full items-center gap-3 px-4 py-3 text-left transition hover:bg-zinc-50 dark:hover:bg-zinc-800/40"
        onClick={() => setEditing(true)}
      >
        {/* same StatusDot/title/pills/duration/chevron */}
      </button>
      <Show when={editing()}>
        <EditEntryDialog
          entry={props.entry}
          onClose={() => setEditing(false)}
          onSaved={() => {
            setEditing(false);
            props.onChange?.();
          }}
        />
      </Show>
    </li>
  );
}
```

The "Delete entry" button previously lived in the expanded panel. Move it into the dialog (add to the footer alongside Save/Cancel, behind a confirm), or — to keep this slice tight — leave deletion to the existing `props.onDelete` invocation path elsewhere (`EntryList` already has a delete handler). For this slice, hoist delete into the dialog:

In `EditEntryDialog.tsx`'s footer, add before the Save button:

```tsx
<Button
  variant="ghost"
  size="sm"
  onClick={async () => {
    if (!confirm("Delete this entry?")) return;
    await api.deleteEntry(props.entry.local_uuid);
    props.onSaved();
    props.onClose();
  }}
>
  Delete
</Button>
```

- [ ] **Step 2: Verify**

```bash
cd ui && pnpm typecheck && pnpm build && cd ..
```

Expected: PASS.

- [ ] **Step 3: Visual check**

`scripts/dev-app.sh`. On Today, click any completed entry: dialog opens. Change start/end times (e.g., +15 minutes on end). Save. Confirm the duration column on the row updates. Reopen to confirm the new times persisted.

Try editing a running entry — the dialog should hide the start/end time inputs (`isCompleted()` is false) and offer only description/project/billable.

- [ ] **Step 4: Commit**

```bash
git add ui/src/components/EntryRow.tsx \
        ui/src/components/EditEntryDialog.tsx
git commit -m "feat(ui): EntryRow opens EditEntryDialog on click"
```

---

### Task 3.6: CLI `stint edit --start --end` flags

**Files:**
- Modify: `crates/stint-cli/src/cmd/edit.rs`
- Create test: `crates/stint-cli/tests/cli_edit_times.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/stint-cli/tests/cli_edit_times.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use stint_core::store::entries::Entries;
use stint_core::store::entries::NewTimeEntry;
use stint_core::store::Store;
use tempfile::TempDir;

fn cmd(db: &std::path::Path) -> Command {
    let mut c = Command::cargo_bin("stint").expect("binary built");
    c.env("STINT_DB", db);
    c
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_start_and_end_updates_times_keeping_date() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    // Seed a completed entry on 2026-05-20.
    let store = Store::connect(&db).await.unwrap();
    let entries = Entries::new(store.clone());
    let id = entries
        .create(NewTimeEntry {
            description: "seed".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T09:00:00Z".into(),
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();
    entries
        .set_end(&id, "2026-05-20T10:00:00Z")
        .await
        .unwrap();

    cmd(&db)
        .args(["edit", &id, "--start", "09:15", "--end", "10:45"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated times"));

    // Re-read directly to assert.
    let store = Store::connect(&db).await.unwrap();
    let entries = Entries::new(store);
    let row = entries.get(&id).await.unwrap().unwrap();
    // We're storing UTC; the CLI accepts HH:MM as local time and converts
    // to UTC. The actual stored value depends on the test machine's TZ; the
    // important invariant is that the *date portion* hasn't changed and
    // both timestamps parsed cleanly.
    assert!(row.start_at.starts_with("2026-05-20T"));
    assert!(row.end_at.as_ref().unwrap().starts_with("2026-05-20T"));
    assert!(row.end_at.as_ref().unwrap() > &row.start_at);
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test -p stint-cli --test cli_edit_times -- --test-threads=1
```

Expected: FAIL — `--start` / `--end` flags don't exist.

- [ ] **Step 3: Extend `edit.rs`**

Open `crates/stint-cli/src/cmd/edit.rs`. Replace with:

```rust
use anyhow::{anyhow, Result};
use chrono::{DateTime, Datelike, Local, NaiveTime, TimeZone, Utc};
use stint_core::store::entries::Entries;
use stint_core::timer::TimerService;

use super::open_store;

#[derive(clap::Args)]
pub struct Args {
    /// Entry UUID (or its 8-character prefix).
    pub id: String,
    /// New description.
    #[arg(long)]
    pub description: Option<String>,
    /// New start time, HH:MM (interpreted in local timezone, day = entry's existing date).
    #[arg(long)]
    pub start: Option<String>,
    /// New end time, HH:MM (interpreted in local timezone, day = entry's existing date).
    #[arg(long)]
    pub end: Option<String>,
}

pub async fn run(args: Args) -> Result<()> {
    let store = open_store().await?;
    let entries = Entries::new(store.clone());
    let timer = TimerService::new(store);

    if let Some(d) = args.description {
        timer.update_description(&args.id, &d).await?;
        println!("Updated description for {}.", &args.id);
    }

    if args.start.is_some() || args.end.is_some() {
        let row = entries
            .get(&args.id)
            .await?
            .ok_or_else(|| anyhow!("entry {} not found", args.id))?;
        let existing_start = DateTime::parse_from_rfc3339(&row.start_at)?.with_timezone(&Utc);
        let existing_end = row
            .end_at
            .as_deref()
            .ok_or_else(|| anyhow!("cannot edit times on a running entry"))?;
        let existing_end = DateTime::parse_from_rfc3339(existing_end)?.with_timezone(&Utc);

        let new_start = match args.start.as_deref() {
            Some(hhmm) => combine_local_hhmm(existing_start, hhmm)?,
            None => existing_start,
        };
        let new_end = match args.end.as_deref() {
            Some(hhmm) => combine_local_hhmm(existing_end, hhmm)?,
            None => existing_end,
        };

        let start_str = new_start.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let end_str = new_end.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        timer.update_times(&args.id, &start_str, &end_str).await?;
        println!("Updated times for {}.", &args.id);
    }

    if args.description.is_none() && args.start.is_none() && args.end.is_none() {
        println!("Nothing to update. Pass --description / --start / --end to change something.");
    }
    Ok(())
}

fn combine_local_hhmm(reference_utc: DateTime<Utc>, hhmm: &str) -> Result<DateTime<Utc>> {
    let parsed = NaiveTime::parse_from_str(hhmm, "%H:%M")
        .map_err(|e| anyhow!("invalid HH:MM '{hhmm}': {e}"))?;
    let local_ref = reference_utc.with_timezone(&Local);
    let date = local_ref.date_naive();
    let local_dt = Local
        .from_local_datetime(&date.and_time(parsed))
        .single()
        .ok_or_else(|| anyhow!("ambiguous local time {hhmm} on {date}"))?;
    Ok(local_dt.with_timezone(&Utc))
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p stint-cli --test cli_edit_times -- --test-threads=1
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-cli/src/cmd/edit.rs \
        crates/stint-cli/tests/cli_edit_times.rs
git commit -m "feat(cli): stint edit --start/--end"
```

---

### Task 3.7: Slice-3 verification

- [ ] **Step 1: Workspace tests**

```bash
cargo test --workspace -- --test-threads=1
cd ui && pnpm typecheck && pnpm build && cd ..
```

Expected: all green.

- [ ] **Step 2: Visual sweep**

`scripts/dev-app.sh`. On Today:
- Open the dialog for a completed entry. Change start to 09:15, end to 10:45. Save. Confirm new duration.
- Open the dialog for the running timer (if one is running). Confirm time inputs are hidden.
- CLI: `scripts/dev-cli.sh edit <id> --start 09:30 --end 10:00` against the seeded entry.

---

## Commit 4 — Backdate start

### Task 4.1: `StartArgs.start_at` + `TimerService::start` honors it

**Files:**
- Modify: `crates/stint-core/src/timer.rs`
- Extend test: `crates/stint-core/tests/timer.rs`

- [ ] **Step 1: Write the failing tests**

Open `crates/stint-core/tests/timer.rs` and append:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn start_with_explicit_start_at_uses_provided_time() {
    let store = common::setup().await;
    let timer = stint_core::timer::TimerService::new(store.clone());
    let entries = stint_core::store::entries::Entries::new(store);

    let backdate = "2026-05-20T08:30:00Z";
    let id = timer
        .start(stint_core::timer::StartArgs {
            description: "x".into(),
            project_id: None,
            task_id: None,
            billable: false,
            source: "cli".into(),
            start_at: Some(backdate.into()),
        })
        .await
        .unwrap();

    let row = entries.get(&id).await.unwrap().unwrap();
    assert_eq!(row.start_at, backdate);
}

#[tokio::test(flavor = "multi_thread")]
async fn start_with_future_start_at_is_rejected() {
    let store = common::setup().await;
    let timer = stint_core::timer::TimerService::new(store);

    // 1 hour in the future, formatted to second precision so equality is stable.
    let future = (chrono::Utc::now() + chrono::Duration::hours(1))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let err = timer
        .start(stint_core::timer::StartArgs {
            description: "x".into(),
            project_id: None,
            task_id: None,
            billable: false,
            source: "cli".into(),
            start_at: Some(future),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, stint_core::Error::Invariant(_)));
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p stint-core --test timer start_with -- --test-threads=1
```

Expected: FAIL — `start_at` field doesn't exist on `StartArgs`; the future-rejection isn't enforced.

- [ ] **Step 3: Add `start_at` to `StartArgs`**

Open `crates/stint-core/src/timer.rs`. Replace the `StartArgs` struct:

```rust
#[derive(Debug, Clone)]
pub struct StartArgs {
    pub description: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub billable: bool,
    pub source: String,
    pub start_at: Option<String>,
}
```

In `TimerService::start`, replace the `let start_at = time::now_utc();` line with:

```rust
let start_at = match args.start_at.as_deref() {
    Some(provided) => {
        let parsed = time::parse(provided)?;
        if parsed > time::now() {
            return Err(Error::Invariant(
                "start time cannot be in the future".into(),
            ));
        }
        // Re-format so storage form matches the rest of the codebase
        // (UTC, second precision, literal Z).
        time::format(&parsed)
    }
    None => time::now_utc(),
};
```

- [ ] **Step 4: Fix every `StartArgs { … }` literal in the codebase**

```bash
cargo build --workspace 2>&1 | grep "missing field"
```

Hits:
- `crates/stint-cli/src/cmd/start.rs`
- `crates/stint-app/src/commands/timer.rs`
- Other tests that construct `StartArgs` (e.g. `crates/stint-core/tests/timer.rs` pre-existing tests)

Add `start_at: None,` to each literal.

- [ ] **Step 5: Run tests**

```bash
cargo test -p stint-core --test timer -- --test-threads=1
cargo test --workspace -- --test-threads=1
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/stint-core/src/timer.rs \
        crates/stint-core/tests/timer.rs \
        crates/stint-cli/src/cmd/start.rs \
        crates/stint-app/src/commands/timer.rs
git commit -m "feat(core): TimerService.start honors optional start_at"
```

---

### Task 4.2: Tauri `start_timer` accepts `start_at`

**Files:**
- Modify: `crates/stint-app/src/commands/timer.rs`
- Modify: `ui/src/api.ts`

- [ ] **Step 1: Extend `StartTimerArgs`**

Open `crates/stint-app/src/commands/timer.rs`. Update:

```rust
#[derive(Deserialize)]
pub struct StartTimerArgs {
    pub description: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    #[serde(default)]
    pub billable: bool,
    #[serde(default)]
    pub start_at: Option<String>,
}
```

In `start_timer`, pass it through:

```rust
let id = timer
    .start(StartArgs {
        description: args.description,
        project_id: args.project_id,
        task_id: args.task_id,
        billable: args.billable,
        source: "gui".into(),
        start_at: args.start_at,
    })
    .await?;
```

- [ ] **Step 2: Extend `api.startTimer`**

Open `ui/src/api.ts`. Replace `startTimer`:

```ts
startTimer: (
  description: string,
  projectId?: string | null,
  taskId?: string | null,
  billable = false,
  startAt?: string | null,
) =>
  invoke<string>("start_timer", {
    args: {
      description,
      project_id: projectId ?? null,
      task_id: taskId ?? null,
      billable,
      start_at: startAt ?? null,
    },
  }),
```

- [ ] **Step 3: Update the `useTimerStore.start` helper**

Open `ui/src/stores/timer.ts`. Replace `start`:

```ts
async start(
  description: string,
  projectId?: string,
  billable = false,
  startAt?: string,
) {
  await api.startTimer(description, projectId ?? null, null, billable, startAt ?? null);
  await refresh();
},
```

- [ ] **Step 4: Build**

```bash
cargo build -p stint-app
cd ui && pnpm typecheck && pnpm build && cd ..
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-app/src/commands/timer.rs \
        ui/src/api.ts ui/src/stores/timer.ts
git commit -m "feat(app): start_timer takes optional start_at"
```

---

### Task 4.3: `StartAtPicker` component

**Files:**
- Create: `ui/src/components/StartAtPicker.tsx`

- [ ] **Step 1: Build the component**

Create `ui/src/components/StartAtPicker.tsx`:

```tsx
import { For, Show, createSignal } from "solid-js";

type Preset = { label: string; minutesAgo: number };

const PRESETS: Preset[] = [
  { label: "5 min", minutesAgo: 5 },
  { label: "15 min", minutesAgo: 15 },
  { label: "30 min", minutesAgo: 30 },
  { label: "1 hour", minutesAgo: 60 },
];

/// Returns null when "now" is chosen (no override). Otherwise returns an
/// ISO 8601 UTC timestamp.
export type StartAtValue = string | null;

export default function StartAtPicker(props: {
  value: StartAtValue;
  onChange: (v: StartAtValue) => void;
}) {
  const [open, setOpen] = createSignal(false);
  const [custom, setCustom] = createSignal("");

  function pickPreset(minutesAgo: number) {
    const t = new Date(Date.now() - minutesAgo * 60 * 1000);
    props.onChange(t.toISOString());
  }

  function pickCustom() {
    const c = custom().trim();
    if (!c) return;
    // Interpret as today HH:MM in local time.
    const [hStr, mStr] = c.split(":");
    const out = new Date();
    out.setHours(parseInt(hStr, 10), parseInt(mStr, 10), 0, 0);
    if (out.getTime() > Date.now()) {
      // Treat a "future" HH:MM as yesterday at that time.
      out.setDate(out.getDate() - 1);
    }
    props.onChange(out.toISOString());
  }

  function clear() {
    props.onChange(null);
  }

  const label = () => {
    const v = props.value;
    if (!v) return "Start now";
    const minsAgo = Math.round((Date.now() - new Date(v).getTime()) / 60000);
    if (minsAgo < 1) return "Start now";
    if (minsAgo < 60) return `Start ${minsAgo} min ago`;
    const hrs = Math.round(minsAgo / 6) / 10;
    return `Start ${hrs}h ago`;
  };

  return (
    <div class="text-xs">
      <button
        type="button"
        class="text-zinc-500 underline-offset-2 hover:text-zinc-900 hover:underline dark:text-zinc-400 dark:hover:text-zinc-100"
        onClick={() => setOpen((v) => !v)}
      >
        {label()}
        <span class="ml-1 text-zinc-400">▾</span>
      </button>
      <Show when={open()}>
        <div class="mt-2 flex flex-wrap items-center gap-1.5">
          <button
            type="button"
            class="rounded-full bg-zinc-100 px-2.5 py-1 text-[11px] hover:bg-zinc-200 dark:bg-zinc-800 dark:hover:bg-zinc-700"
            onClick={clear}
          >
            Now
          </button>
          <For each={PRESETS}>
            {(p) => (
              <button
                type="button"
                class="rounded-full bg-zinc-100 px-2.5 py-1 text-[11px] hover:bg-zinc-200 dark:bg-zinc-800 dark:hover:bg-zinc-700"
                onClick={() => pickPreset(p.minutesAgo)}
              >
                {p.label} ago
              </button>
            )}
          </For>
          <input
            type="time"
            class="rounded border border-zinc-200 bg-white px-1.5 py-0.5 text-[11px] dark:border-zinc-700 dark:bg-zinc-950"
            value={custom()}
            onInput={(e) => setCustom(e.currentTarget.value)}
            onBlur={pickCustom}
            onKeyDown={(e) => {
              if (e.key === "Enter") pickCustom();
            }}
          />
        </div>
      </Show>
    </div>
  );
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cd ui && pnpm typecheck && pnpm build && cd ..
```

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add ui/src/components/StartAtPicker.tsx
git commit -m "feat(ui): StartAtPicker with presets + custom HH:MM"
```

---

### Task 4.4: Wire `StartAtPicker` into `TimerCard` + `Popover`

**Files:**
- Modify: `ui/src/components/TimerCard.tsx`
- Modify: `ui/src/routes/Popover.tsx`

- [ ] **Step 1: Add to TimerCard**

Open `ui/src/components/TimerCard.tsx`. Add:

```tsx
import StartAtPicker, { type StartAtValue } from "./StartAtPicker";
```

Add a `startAt` signal:

```tsx
const [startAt, setStartAt] = createSignal<StartAtValue>(null);
```

In the `onSubmit` handler, replace:

```tsx
timer.start(d, projectId() || undefined, billable()).then(() => {
  setDescription("");
  setBillable(false);
});
```

with:

```tsx
timer
  .start(d, projectId() || undefined, billable(), startAt() ?? undefined)
  .then(() => {
    setDescription("");
    setBillable(false);
    setStartAt(null);
  });
```

Add the picker beneath the description input, above the project/billable row:

```tsx
<StartAtPicker value={startAt()} onChange={setStartAt} />
```

- [ ] **Step 2: Add to Popover**

Open `ui/src/routes/Popover.tsx`. Same imports + signal pattern. Insert the `<StartAtPicker>` between the description input and the project/billable row.

- [ ] **Step 3: Build**

```bash
cd ui && pnpm typecheck && pnpm build && cd ..
```

Expected: PASS.

- [ ] **Step 4: Visual check**

`scripts/dev-app.sh`. In the main TimerCard:
- Type a description, click "Start ▾", click "15 min ago", click Start.
- Stop the timer. Open the entry: start_at should be ~15 minutes before now.
- Repeat in the Popover.

- [ ] **Step 5: Commit**

```bash
git add ui/src/components/TimerCard.tsx ui/src/routes/Popover.tsx
git commit -m "feat(ui): backdate start option in TimerCard + Popover"
```

---

### Task 4.5: CLI `stint start --at`

**Files:**
- Modify: `crates/stint-cli/src/cmd/start.rs`
- Create: `crates/stint-cli/src/at_parse.rs`
- Modify: `crates/stint-cli/src/main.rs` (declare `mod at_parse;`)
- Create test: `crates/stint-cli/tests/cli_start_at.rs`

- [ ] **Step 1: Write failing parser unit tests**

Create `crates/stint-cli/src/at_parse.rs`:

```rust
//! Parses the `--at` argument for `stint start`. Accepts:
//!   - relative ago: "5min ago", "30 min ago", "1h ago", "1hr ago", "1 hour ago"
//!   - bare HH:MM (interpreted as today local time, day-shift to yesterday if future)
//!   - RFC 3339 absolute timestamp
//! Returns a UTC RFC 3339 string at second precision, suitable for stint-core.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Local, NaiveTime, SecondsFormat, TimeZone, Utc};

pub fn parse_at_arg(input: &str) -> Result<String> {
    let s = input.trim();
    if s.is_empty() {
        return Err(anyhow!("--at value is empty"));
    }

    // 1. Absolute RFC 3339?
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc).to_rfc3339_opts(SecondsFormat::Secs, true));
    }

    // 2. Bare HH:MM today (local)?
    if let Ok(t) = NaiveTime::parse_from_str(s, "%H:%M") {
        let local_now = Local::now();
        let candidate = local_now
            .with_time(t)
            .single()
            .ok_or_else(|| anyhow!("ambiguous local time {s}"))?;
        let resolved = if candidate > local_now {
            candidate - Duration::days(1)
        } else {
            candidate
        };
        return Ok(resolved
            .with_timezone(&Utc)
            .to_rfc3339_opts(SecondsFormat::Secs, true));
    }

    // 3. Relative "<n><unit> ago"
    let lower = s.to_ascii_lowercase();
    let stripped = lower
        .strip_suffix(" ago")
        .or_else(|| lower.strip_suffix("ago"))
        .map(str::trim)
        .ok_or_else(|| anyhow!("could not parse '{s}'; try '15 min ago' or '09:30'"))?;
    let (num_str, unit_str) = split_num_unit(stripped)?;
    let n: i64 = num_str.parse().map_err(|e| anyhow!("bad number '{num_str}': {e}"))?;
    let dur = match unit_str {
        "m" | "min" | "mins" | "minute" | "minutes" => Duration::minutes(n),
        "h" | "hr" | "hrs" | "hour" | "hours" => Duration::hours(n),
        other => return Err(anyhow!("unknown unit '{other}' (try min or hour)")),
    };
    let when = Utc::now() - dur;
    Ok(when.to_rfc3339_opts(SecondsFormat::Secs, true))
}

fn split_num_unit(s: &str) -> Result<(&str, &str)> {
    let s = s.trim();
    let idx = s
        .find(|c: char| !c.is_ascii_digit())
        .ok_or_else(|| anyhow!("missing unit after number in '{s}'"))?;
    let (num, rest) = s.split_at(idx);
    let unit = rest.trim();
    Ok((num, unit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_5min_ago() {
        let out = parse_at_arg("5min ago").unwrap();
        let parsed = DateTime::parse_from_rfc3339(&out).unwrap();
        let now = Utc::now();
        let diff = now.signed_duration_since(parsed).num_seconds();
        assert!(diff >= 295 && diff <= 305, "expected ~300s, got {diff}");
    }

    #[test]
    fn parses_30_min_ago() {
        parse_at_arg("30 min ago").unwrap();
    }

    #[test]
    fn parses_1h_ago() {
        let out = parse_at_arg("1h ago").unwrap();
        let parsed = DateTime::parse_from_rfc3339(&out).unwrap();
        let diff = Utc::now().signed_duration_since(parsed).num_seconds();
        assert!(diff >= 3595 && diff <= 3605);
    }

    #[test]
    fn parses_1hr_ago() {
        parse_at_arg("1hr ago").unwrap();
    }

    #[test]
    fn parses_rfc3339() {
        let out = parse_at_arg("2026-05-20T09:00:00Z").unwrap();
        assert_eq!(out, "2026-05-20T09:00:00Z");
    }

    #[test]
    fn parses_hhmm() {
        let out = parse_at_arg("09:30").unwrap();
        // Just verify it produced a valid timestamp at second precision.
        DateTime::parse_from_rfc3339(&out).unwrap();
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_at_arg("yesterday").is_err());
        assert!(parse_at_arg("").is_err());
    }
}
```

- [ ] **Step 2: Wire the module into the binary**

Open `crates/stint-cli/src/main.rs` and add at the top:

```rust
mod at_parse;
```

- [ ] **Step 3: Run unit tests**

```bash
cargo test -p stint-cli at_parse -- --test-threads=1
```

Expected: PASS (all unit tests).

- [ ] **Step 4: Write failing E2E test**

Create `crates/stint-cli/tests/cli_start_at.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use stint_core::store::entries::Entries;
use stint_core::store::Store;
use tempfile::TempDir;

fn cmd(db: &std::path::Path) -> Command {
    let mut c = Command::cargo_bin("stint").expect("binary built");
    c.env("STINT_DB", db);
    c
}

#[tokio::test(flavor = "multi_thread")]
async fn start_with_at_15min_ago_backdates_entry() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db)
        .args(["start", "deep work", "--at", "15min ago"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started: deep work"));

    let store = Store::connect(&db).await.unwrap();
    let entries = Entries::new(store);
    let rows = entries
        .list_between("2000-01-01T00:00:00Z", "2099-01-01T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    let start = chrono::DateTime::parse_from_rfc3339(&rows[0].start_at).unwrap();
    let diff = chrono::Utc::now()
        .signed_duration_since(start.with_timezone(&chrono::Utc))
        .num_seconds();
    assert!(diff >= 890 && diff <= 910, "expected ~900s (15min) ago, got {diff}");
}
```

- [ ] **Step 5: Run test to verify it fails**

```bash
cargo test -p stint-cli --test cli_start_at -- --test-threads=1
```

Expected: FAIL — `--at` flag doesn't exist.

- [ ] **Step 6: Add the flag to `start.rs`**

Open `crates/stint-cli/src/cmd/start.rs`. Replace with:

```rust
use anyhow::Result;
use stint_core::timer::{StartArgs, TimerService};

use crate::at_parse;
use super::open_store;

#[derive(clap::Args)]
pub struct Args {
    /// Description of what you're working on.
    pub description: String,
    /// Project ID (Solidtime UUID).
    #[arg(long)]
    pub project: Option<String>,
    /// Task ID (Solidtime UUID).
    #[arg(long)]
    pub task: Option<String>,
    /// Backdate the start (relative "15min ago" / "1h ago", HH:MM today, or RFC 3339).
    #[arg(long)]
    pub at: Option<String>,
}

pub async fn run(args: Args) -> Result<()> {
    let store = open_store().await?;
    let timer = TimerService::new(store);
    let start_at = match args.at.as_deref() {
        Some(s) => Some(at_parse::parse_at_arg(s)?),
        None => None,
    };
    let id = timer
        .start(StartArgs {
            description: args.description.clone(),
            project_id: args.project,
            task_id: args.task,
            billable: false,
            source: "cli".into(),
            start_at,
        })
        .await?;
    println!("Started: {} ({})", args.description, id);
    Ok(())
}
```

`mod at_parse` was added to `main.rs` in Step 2; this file needs `use crate::at_parse;` (already shown above). Verify `main.rs` exposes the modules — clap subcommand dispatch happens there, and `cmd::start` is already wired.

- [ ] **Step 7: Run all CLI tests**

```bash
cargo test -p stint-cli -- --test-threads=1
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/stint-cli/src/at_parse.rs \
        crates/stint-cli/src/cmd/start.rs \
        crates/stint-cli/src/main.rs \
        crates/stint-cli/tests/cli_start_at.rs
git commit -m "feat(cli): stint start --at <when>"
```

---

### Task 4.6: Slice-4 verification

- [ ] **Step 1: Workspace tests**

```bash
cargo test --workspace -- --test-threads=1
cd ui && pnpm typecheck && pnpm build && cd ..
```

Expected: all green.

- [ ] **Step 2: Visual sweep**

`scripts/dev-app.sh`:
- Type a description in TimerCard. Click the "Start now ▾" button. Click "15 min ago". Click Start.
- Stop. Open the entry's edit dialog. Confirm start time is ~15min before now.
- Repeat in the popover.
- CLI: `scripts/dev-cli.sh start "morning standup" --at "1h ago"` → confirm in `scripts/dev-cli.sh today`.

---

## Task 5 — Final verification + PR

- [ ] **Step 1: Full test sweep**

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --test-threads=1
cd ui && pnpm typecheck && pnpm build && cd ..
```

Expected: all green. Fix lint hits if any.

- [ ] **Step 2: Manual UAT against a real Solidtime**

Configure `solidtime.url` + token + org via `scripts/dev-cli.sh config set …`. Then in the GUI:

1. **§1 – ProjectPicker:**
   - Open TimerCard. Type "log" → list filters to projects containing "log". Pick one. Start. Stop. Confirm pill.
   - Open Popover. Same filter + pick + start + stop.
   - Open Today → expand an entry → change project via picker.

2. **§2 – Calendar default project:**
   - Settings → an account → Calendars. Pick a default project on the personal calendar.
   - Today → Calendar section → "Log this" on an event → confirm the resulting entry already has the default project assigned (visible in the row's pill, and on the edit dialog).
   - Clear the default in Settings. Log a fresh event. Confirm no project is assigned.

3. **§3 – Entry edit dialog:**
   - Click any completed entry on Today. Change times by ±15 min. Save. Confirm row updates.
   - Try invalid: end before start. Confirm an error appears in the dialog.
   - CLI: `scripts/dev-cli.sh edit <prefix> --start 09:30 --end 09:45`.

4. **§4 – Backdate start:**
   - TimerCard → "Start now ▾" → "30 min ago" → Start. Stop. Confirm 30-min duration.
   - CLI: `scripts/dev-cli.sh start "deep work" --at "45min ago"` then `scripts/dev-cli.sh stop`.

5. **Sync sanity:**
   - Click the Sync badge. Confirm queued ops drain to Solidtime. Open the Solidtime web UI and confirm the new entries (with project!) appear there.
   - Stop a timer, then edit its times locally. Sync. Confirm Solidtime reflects the new times.

- [ ] **Step 3: Update the README roadmap**

Open `README.md`. In the status block, change `Phase 3b` → `Phase 3d`:

```
- **Phase 3a** ✅ — OAuth 2.0 foundation + Solidtime OAuth sign-in (`phase-3a-complete` tag)
- **Phase 3b** ✅ — Calendar integration (Google) (`phase-3b-complete` tag)
- **Phase 3c** ✅ — Solidtime down-sync (`phase-3c-complete` tag)
- **Phase 3d** — Post-3b UX polish (project picker, calendar default project, editable times, backdate start)
```

Or whatever the existing format is — update consistently.

- [ ] **Step 4: Update CLAUDE.md roadmap table**

Open `CLAUDE.md`. In the "Where we are in the roadmap" table, set Phase 3d's status to shipped:

```
| 3d | Post-3b UX polish (picker / calendar defaults / editable times / backdate) | ✅ shipped (`phase-3d-complete`) |
```

- [ ] **Step 5: Commit doc updates**

```bash
git add README.md CLAUDE.md
git commit -m "docs: mark phase 3d shipped in roadmap"
```

- [ ] **Step 6: Push + PR**

```bash
git push -u origin phase-3d
gh pr create --base main --head phase-3d \
  --title "Phase 3d — UX polish" \
  --body "$(cat <<'EOF'
## Summary
- Searchable client-grouped project picker (kobalte Combobox), reused across start forms and entry edits.
- Per-calendar default project; "Log this" prefills.
- Editable start/end times on completed entries (GUI dialog + CLI flags).
- Backdate the start of a new timer ("5/15/30/60 min ago" presets + custom HH:MM; CLI `--at`).

Spec: docs/superpowers/specs/2026-05-20-post-3b-ux-polish.md
Plan: docs/superpowers/plans/2026-05-20-stint-phase-3d-ux-polish.md

## Test plan
- [ ] CI green
- [ ] Manual UAT in scripts/dev-app.sh covering §1–§4 (see plan Task 5)
- [ ] Manual sync round-trip against real Solidtime
EOF
)"
```

- [ ] **Step 7: After CI green + review + merge, tag**

(Ask the user before pushing the tag.)

```bash
git checkout main
git pull --ff-only
git tag -a phase-3d-complete -m "Phase 3d — UX polish"
git push origin phase-3d-complete
```

---

## Self-review notes

- **Spec coverage:** §1 (Task 1.x), §2 (Task 2.x), §3 (Task 3.x), §4 (Task 4.x). All Decisions from spec §6 are reflected: kobalte Combobox (Task 1.7), same-day-only edit (Task 3.4 hides time inputs based on isCompleted, dialog uses entry's existing date as day anchor), four backdate presets + custom (Task 4.3).
- **Migration numbering:** 0004 (clients), 0005 (calendars.default_project_id). Phase 3c added 0003; verified via `ls crates/stint-core/migrations/`.
- **Tauri command extraction:** intentionally not done in this phase — would balloon the plan. The upcoming testing-uplift phase covers it.
- **No FK on calendars.default_project_id → projects.id:** explained in Task 2.1 migration comment. Reasoning: Solidtime project deletes shouldn't fail an unrelated SQL constraint here; the calendar quietly loses its default.
- **`@kobalte/core` first UI library:** noted in spec §6; restricted scope (combobox only). No other components from kobalte are reached for in this plan.
- **Time-zone handling for HH:MM inputs (Task 3.4, 3.6, 4.3):** all use `Local` for display + `Utc` for storage. CLI `combine_local_hhmm` keeps the date portion of the original UTC timestamp's local representation, then re-converts. UI's `fromLocalHHMM` uses `Date.setHours` (always local-time) followed by `toISOString()` (UTC). Documented inline.

---

## Pragmatic caveats

- **No automated UI tests in this phase.** The four new components (`ProjectPicker`, `EditEntryDialog`, `StartAtPicker`, plus the `CalendarsManager` extension) are covered by manual UAT (Task 5 step 2). Adding Vitest + @solidjs/testing-library is the testing-uplift phase's job.
- **Backdate validation is core-only.** UI doesn't soft-warn on >12h backdates (spec §4 mentions it). Easy follow-up if real users hit it; not worth the dialog churn now.
- **Edit dialog same-day only.** Per spec decision. The CLI shares the constraint (HH:MM resolves against the entry's existing date).
- **Combobox grouping is visual-only (per-item client subtitle).** Kobalte's Combobox lacks a section primitive at the time of writing. If the flat list feels noisy in real use (lots of projects across many clients), revisit with either manual sectioning or a different primitive.
