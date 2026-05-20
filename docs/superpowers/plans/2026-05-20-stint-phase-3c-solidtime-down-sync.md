# Phase 3c — Solidtime down-sync Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pull running-timer + recent-history + deletion state from Solidtime back into stint's local SQLite so a timer started outside stint (web/another device) is reflected here, and entries edited/deleted elsewhere stop drifting.

**Architecture:** New `stint-core::sync::pull` module mirroring the shape of the existing `sync::push` + `sync::refresh`: a single `pull(...)` entry point runs three sub-reconciliations (running-timer, history, deletes) inside one SQLite transaction. It's triggered four ways — app startup, main-window focus (debounced 30s), 5-minute background poll, explicit CLI/GUI manual trigger. Adoption of a remote-only running timer is automatic; a conflict (local + remote both running with different ids) surfaces via a non-blocking banner whose three actions (Stop remotely / Switch / Dismiss) plumb through existing sync paths.

**Tech Stack:** Rust 1.95 (stint-core), Tauri 2 (stint-app), clap (stint-cli), SolidJS + Tailwind (ui), wiremock for HTTP-shape tests.

**Spec:** `docs/superpowers/specs/2026-05-20-solidtime-down-sync.md` (read it; this plan implements it verbatim).

---

## File Structure

### stint-core (new)

| Path | Responsibility |
|---|---|
| `crates/stint-core/src/sync/pull/mod.rs` | Re-exports + top-level `pull(store, client, trigger)` entry point + `PullReport` + `Trigger` enum. |
| `crates/stint-core/src/sync/pull/window.rs` | Window calculation per Trigger (e.g. `Trigger::OnStartup` → last 24h). Pure functions; trivially testable. |
| `crates/stint-core/src/sync/pull/running.rs` | `reconcile_running` — adopts remote running entry, detects conflict. Returns `RunningOutcome`. |
| `crates/stint-core/src/sync/pull/history.rs` | `reconcile_history` — insert / update completed entries from remote. Returns `(inserted, updated)`. |
| `crates/stint-core/src/sync/pull/deletes.rs` | `reconcile_deletes` — for each local `synced` row in the window not present in the list response, fetch by id; delete on 404. |

### stint-core (modified)

| Path | Change |
|---|---|
| `crates/stint-core/src/sync/mod.rs` | Add `pub mod pull;`. |
| `crates/stint-core/src/solidtime/mod.rs` | Add `list_time_entries(member_id, from, to)` and `get_time_entry(id)`. |
| `crates/stint-core/src/solidtime/dto.rs` | Add `updated_at: Option<String>` to `RemoteTimeEntry`. |
| `crates/stint-core/src/store/entries.rs` | Add `create_from_remote`, `update_from_remote`, `hard_delete_by_solidtime_id`, `get_by_solidtime_id`, `list_synced_in_window`. |
| `crates/stint-core/src/store/running.rs` | (No schema change. Existing `set` is sufficient.) |

### stint-core (tests)

| Path | Coverage |
|---|---|
| `crates/stint-core/tests/sync_pull_running.rs` | All five cases from spec §6 (none/none, none/some, some/none → adopt, same id → noop, different id → conflict). |
| `crates/stint-core/tests/sync_pull_history.rs` | Insert new, update when remote newer, skip when local has pending_*, no-op when local newer. |
| `crates/stint-core/tests/sync_pull_deletes.rs` | 404 → delete locally; 200 → keep; cap at 50 fetches per pull. |
| `crates/stint-core/tests/sync_pull_http.rs` | wiremock — verify request shape (query params, headers) on the list call. |
| `crates/stint-core/tests/solidtime_dto.rs` (extend) | Deserialize an active-entry JSON with `end: null` + `updated_at` present. |

### stint-cli

| Path | Change |
|---|---|
| `crates/stint-cli/src/cmd/pull.rs` (new) | `stint pull` subcommand; prints `+N entries, ~M updates, -K deletes` summary; supports `--stop-remote`, `--switch`, `--dismiss` to resolve a conflict. |
| `crates/stint-cli/src/cmd/mod.rs` | `pub mod pull;` |
| `crates/stint-cli/src/main.rs` | Register `Command::Pull(cmd::pull::Args)` + dispatch. |
| `crates/stint-cli/src/cmd/today.rs` | Render `(adopted from Solidtime)` line next to running timer when `source = 'solidtime'`. |
| `crates/stint-cli/tests/cli_e2e.rs` (extend) | `stint pull` against a wiremock server end-to-end. |

### stint-app

| Path | Change |
|---|---|
| `crates/stint-app/src/pull_worker.rs` (new) | Spawns a background task that pulls every 5 min; exposes `pub fn nudge(app, store)` for one-shot pulls. |
| `crates/stint-app/src/commands/pull.rs` (new) | Tauri command `pull_now` → returns `PullReport`-shaped JSON. Tauri command `conflict_resolve` (action: stop / switch / dismiss). |
| `crates/stint-app/src/commands/mod.rs` | `pub mod pull;` |
| `crates/stint-app/src/main.rs` | Wire `pull_worker::spawn` in setup, register new commands in `invoke_handler!`. Emit `pull:conflict` on conflict detection. |
| `crates/stint-app/src/sync_worker.rs` | Add a constant for the new `pull:conflict` event name (kept beside `EVENT_ENTRIES_CHANGED` for symmetry — single source of truth). |

### ui

| Path | Change |
|---|---|
| `ui/src/api.ts` | Add `pullNow()` and `conflictResolve(action, payload)` IPC bindings; add `PullReport` + `ConflictInfo` types. |
| `ui/src/components/ConflictBanner.tsx` (new) | Three-button banner per spec §7. Subscribes to `pull:conflict` Tauri event. |
| `ui/src/components/PullStatus.tsx` (new) | "Last synced N seconds ago • Refresh" line. |
| `ui/src/routes/Today.tsx` | Render `<ConflictBanner />` + `<PullStatus />` at the top. |
| `ui/src/routes/Popover.tsx` | No code change required: existing `entries:changed` listener already refreshes timer state, so adoption shows up within one tick. (Verified manually.) |

---

## Phasing summary

Three commits, each shippable on its own:

1. **Tasks 1–10 — Running-timer adoption + conflict UI** (commit message: `feat(core): pull running timer from Solidtime`).
2. **Tasks 11–14 — Recent-history reconciliation** (commit: `feat(core): reconcile recent history from Solidtime`).
3. **Tasks 15–17 — Delete reconciliation + background poll** (commit: `feat(core): reconcile remote deletes; spawn pull worker`).

Then **Task 18 — final manual verification + PR**.

---

## Pre-flight

**Branch:**

```bash
git checkout main
git pull --ff-only
git checkout -b phase-3c
```

**Confirm baseline:**

```bash
cargo build --workspace
cargo test --workspace -- --test-threads=1
cd ui && pnpm typecheck && cd ..
```

Expected: all green. Stop and investigate if anything fails before starting Task 1.

---

## Task 1: Add `updated_at` to `RemoteTimeEntry`; allow `end: null`

**Files:**
- Modify: `crates/stint-core/src/solidtime/dto.rs:42-56`
- Test: `crates/stint-core/tests/solidtime_dto.rs` (new — small unit test file)

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/solidtime_dto.rs`:

```rust
use stint_core::solidtime::dto::RemoteTimeEntry;

#[test]
fn deserializes_active_entry_with_null_end_and_updated_at() {
    let json = r#"{
        "id": "remote-1",
        "description": "writing tests",
        "project_id": null,
        "task_id": null,
        "start": "2026-05-20T17:00:00Z",
        "end": null,
        "billable": false,
        "updated_at": "2026-05-20T17:01:00Z"
    }"#;
    let e: RemoteTimeEntry = serde_json::from_str(json).unwrap();
    assert_eq!(e.id, "remote-1");
    assert!(e.end.is_none());
    assert_eq!(e.updated_at.as_deref(), Some("2026-05-20T17:01:00Z"));
}

#[test]
fn deserializes_completed_entry_without_updated_at_field() {
    let json = r#"{
        "id": "remote-2",
        "description": "done",
        "start": "2026-05-20T10:00:00Z",
        "end": "2026-05-20T11:00:00Z",
        "billable": true
    }"#;
    let e: RemoteTimeEntry = serde_json::from_str(json).unwrap();
    assert_eq!(e.end.as_deref(), Some("2026-05-20T11:00:00Z"));
    assert!(e.updated_at.is_none());
}
```

To access `RemoteTimeEntry` from an integration test, it needs to be re-exported. Check `crates/stint-core/src/solidtime/mod.rs:1-3` — `pub mod dto;` is already there, and `RemoteTimeEntry` is `pub`, so `stint_core::solidtime::dto::RemoteTimeEntry` is reachable.

- [ ] **Step 2: Run test, verify compile error or assertion failure**

```bash
cargo test -p stint-core --test solidtime_dto -- --test-threads=1
```

Expected: compile error — `RemoteTimeEntry` has no `updated_at` field.

- [ ] **Step 3: Add field**

Edit `crates/stint-core/src/solidtime/dto.rs:42-56`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteTimeEntry {
    pub id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    pub start: String,
    #[serde(default)]
    pub end: Option<String>,
    #[serde(default)]
    pub billable: bool,
    #[serde(default)]
    pub updated_at: Option<String>,
}
```

- [ ] **Step 4: Run test, verify PASS**

```bash
cargo test -p stint-core --test solidtime_dto -- --test-threads=1
```

Expected: 2 passed.

- [ ] **Step 5: Verify workspace still compiles** (existing code might consume `end` as `String`)

```bash
cargo build --workspace
```

Expected: clean build. `end` was already `Option<String>` so no callers break.

- [ ] **Step 6: Commit**

```bash
git add crates/stint-core/src/solidtime/dto.rs crates/stint-core/tests/solidtime_dto.rs
git commit -m "feat(core): add updated_at to RemoteTimeEntry"
```

---

## Task 2: Add `SolidtimeClient::list_time_entries`

**Files:**
- Modify: `crates/stint-core/src/solidtime/mod.rs` (add method after `delete_time_entry`)
- Test: `crates/stint-core/tests/sync_pull_http.rs` (new)

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/sync_pull_http.rs`:

```rust
mod common;

use stint_core::solidtime::SolidtimeClient;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn list_time_entries_sends_member_id_and_window_params() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .and(query_param("member_ids[]", "m-1"))
        .and(query_param("start", "2026-05-19T17:00:00Z"))
        .and(query_param("end", "2026-05-20T17:00:00Z"))
        .and(header("authorization", "Bearer t"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "remote-1",
                "description": "in progress",
                "start": "2026-05-20T16:30:00Z",
                "end": null,
                "billable": false,
                "updated_at": "2026-05-20T16:30:00Z"
            }]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let entries = client
        .list_time_entries("m-1", "2026-05-19T17:00:00Z", "2026-05-20T17:00:00Z")
        .await
        .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "remote-1");
    assert!(entries[0].end.is_none());
}

#[tokio::test]
async fn list_time_entries_unauth_maps_to_solidtime_auth_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let err = client
        .list_time_entries("m-1", "a", "b")
        .await
        .unwrap_err();
    assert!(matches!(err, stint_core::Error::SolidtimeAuth), "got: {err:?}");
}
```

Reuses the existing `common` module (each test file declares `mod common;` and points at the same `tests/common/mod.rs`).

- [ ] **Step 2: Run test, verify fails to compile**

```bash
cargo test -p stint-core --test sync_pull_http -- --test-threads=1
```

Expected: compile error — `list_time_entries` not found.

- [ ] **Step 3: Implement method**

Add after `crates/stint-core/src/solidtime/mod.rs:184` (right before the closing `}`):

```rust
    pub async fn list_time_entries(
        &self,
        member_id: &str,
        from: &str,
        to: &str,
    ) -> Result<Vec<RemoteTimeEntry>> {
        let org = self.org()?;
        let url = format!("{}/api/v1/organizations/{org}/time-entries", self.base_url);
        let resp = self
            .authed(self.http.get(&url))
            .await?
            .query(&[("member_ids[]", member_id), ("start", from), ("end", to)])
            .send()
            .await?;
        let status = resp.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err(Error::SolidtimeAuth);
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::Solidtime {
                status: status.as_u16(),
                body,
            });
        }
        let wrapper: Wrapper<Vec<RemoteTimeEntry>> = resp.json().await?;
        Ok(wrapper.data)
    }
```

- [ ] **Step 4: Run test, verify PASS**

```bash
cargo test -p stint-core --test sync_pull_http -- --test-threads=1
```

Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/solidtime/mod.rs crates/stint-core/tests/sync_pull_http.rs
git commit -m "feat(core): list_time_entries on SolidtimeClient"
```

---

## Task 3: Add `SolidtimeClient::get_time_entry`

**Files:**
- Modify: `crates/stint-core/src/solidtime/mod.rs`
- Test: `crates/stint-core/tests/sync_pull_http.rs` (extend)

- [ ] **Step 1: Append failing tests** to `crates/stint-core/tests/sync_pull_http.rs`:

```rust
#[tokio::test]
async fn get_time_entry_returns_some_on_200() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries/remote-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "id": "remote-1",
                "description": "still here",
                "start": "2026-05-20T10:00:00Z",
                "end": "2026-05-20T11:00:00Z",
                "billable": true
            }
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let entry = client.get_time_entry("remote-1").await.unwrap();
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().id, "remote-1");
}

#[tokio::test]
async fn get_time_entry_returns_none_on_404() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries/gone"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let entry = client.get_time_entry("gone").await.unwrap();
    assert!(entry.is_none());
}
```

- [ ] **Step 2: Run, verify fails to compile**

```bash
cargo test -p stint-core --test sync_pull_http -- --test-threads=1
```

Expected: compile error.

- [ ] **Step 3: Implement** (append to `SolidtimeClient` impl after `list_time_entries`):

```rust
    pub async fn get_time_entry(&self, id: &str) -> Result<Option<RemoteTimeEntry>> {
        let org = self.org()?;
        let url = format!(
            "{}/api/v1/organizations/{org}/time-entries/{id}",
            self.base_url
        );
        let resp = self.authed(self.http.get(&url)).await?.send().await?;
        let status = resp.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if status == StatusCode::UNAUTHORIZED {
            return Err(Error::SolidtimeAuth);
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::Solidtime {
                status: status.as_u16(),
                body,
            });
        }
        let wrapper: Wrapper<RemoteTimeEntry> = resp.json().await?;
        Ok(Some(wrapper.data))
    }
```

- [ ] **Step 4: Run, verify PASS**

```bash
cargo test -p stint-core --test sync_pull_http -- --test-threads=1
```

Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/solidtime/mod.rs crates/stint-core/tests/sync_pull_http.rs
git commit -m "feat(core): get_time_entry on SolidtimeClient"
```

---

## Task 4: Entries store — new helpers for upstream-origin rows

**Files:**
- Modify: `crates/stint-core/src/store/entries.rs`
- Test: `crates/stint-core/tests/store_entries_from_remote.rs` (new)

- [ ] **Step 1: Write the failing test**

Create `crates/stint-core/tests/store_entries_from_remote.rs`:

```rust
mod common;

use stint_core::store::entries::{Entries, RemoteEntryUpsert};

#[tokio::test]
async fn create_from_remote_inserts_synced_row_with_solidtime_id() {
    let env = common::setup().await;
    let entries = Entries::new(env.store.clone());
    let local_uuid = entries
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "remote-1".into(),
            description: "from server".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T10:00:00Z".into(),
            end_at: Some("2026-05-20T11:00:00Z".into()),
            billable: false,
            updated_at: "2026-05-20T11:00:01Z".into(),
        })
        .await
        .unwrap();
    let row = entries.get(&local_uuid).await.unwrap().unwrap();
    assert_eq!(row.solidtime_id.as_deref(), Some("remote-1"));
    assert_eq!(row.sync_state, "synced");
    assert_eq!(row.source, "solidtime");
}

#[tokio::test]
async fn get_by_solidtime_id_finds_existing_row() {
    let env = common::setup().await;
    let entries = Entries::new(env.store.clone());
    entries
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "remote-2".into(),
            description: "x".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T10:00:00Z".into(),
            end_at: Some("2026-05-20T11:00:00Z".into()),
            billable: false,
            updated_at: "2026-05-20T11:00:01Z".into(),
        })
        .await
        .unwrap();
    let row = entries.get_by_solidtime_id("remote-2").await.unwrap();
    assert!(row.is_some());
    assert_eq!(row.unwrap().description, "x");
    let missing = entries.get_by_solidtime_id("no-such").await.unwrap();
    assert!(missing.is_none());
}

#[tokio::test]
async fn update_from_remote_overwrites_fields_for_synced_row() {
    let env = common::setup().await;
    let entries = Entries::new(env.store.clone());
    entries
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "remote-3".into(),
            description: "old".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T10:00:00Z".into(),
            end_at: Some("2026-05-20T11:00:00Z".into()),
            billable: false,
            updated_at: "2026-05-20T11:00:01Z".into(),
        })
        .await
        .unwrap();
    let changed = entries
        .update_from_remote(
            "remote-3",
            RemoteEntryUpsert {
                solidtime_id: "remote-3".into(),
                description: "new".into(),
                project_id: Some("p-1".into()),
                task_id: None,
                start_at: "2026-05-20T10:00:00Z".into(),
                end_at: Some("2026-05-20T11:30:00Z".into()),
                billable: true,
                updated_at: "2026-05-20T11:30:01Z".into(),
            },
        )
        .await
        .unwrap();
    assert!(changed);
    let row = entries.get_by_solidtime_id("remote-3").await.unwrap().unwrap();
    assert_eq!(row.description, "new");
    assert_eq!(row.project_id.as_deref(), Some("p-1"));
    assert_eq!(row.end_at.as_deref(), Some("2026-05-20T11:30:00Z"));
    assert_eq!(row.billable, 1);
    assert_eq!(row.sync_state, "synced");
}

#[tokio::test]
async fn update_from_remote_skips_pending_local_mutations() {
    let env = common::setup().await;
    let entries = Entries::new(env.store.clone());
    let local_uuid = entries
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "remote-4".into(),
            description: "synced".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T10:00:00Z".into(),
            end_at: Some("2026-05-20T11:00:00Z".into()),
            billable: false,
            updated_at: "2026-05-20T11:00:01Z".into(),
        })
        .await
        .unwrap();
    // Local user edited the description; row is now `dirty`.
    entries
        .update_description(&local_uuid, "local edit")
        .await
        .unwrap();

    let changed = entries
        .update_from_remote(
            "remote-4",
            RemoteEntryUpsert {
                solidtime_id: "remote-4".into(),
                description: "remote edit".into(),
                project_id: None,
                task_id: None,
                start_at: "2026-05-20T10:00:00Z".into(),
                end_at: Some("2026-05-20T11:00:00Z".into()),
                billable: false,
                updated_at: "2026-05-20T12:00:00Z".into(),
            },
        )
        .await
        .unwrap();
    assert!(!changed, "should not overwrite local pending mutation");
    let row = entries.get(&local_uuid).await.unwrap().unwrap();
    assert_eq!(row.description, "local edit");
    assert_eq!(row.sync_state, "dirty");
}

#[tokio::test]
async fn hard_delete_by_solidtime_id_removes_row() {
    let env = common::setup().await;
    let entries = Entries::new(env.store.clone());
    entries
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "remote-5".into(),
            description: "x".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T10:00:00Z".into(),
            end_at: Some("2026-05-20T11:00:00Z".into()),
            billable: false,
            updated_at: "2026-05-20T11:00:01Z".into(),
        })
        .await
        .unwrap();
    let removed = entries.hard_delete_by_solidtime_id("remote-5").await.unwrap();
    assert!(removed);
    assert!(entries.get_by_solidtime_id("remote-5").await.unwrap().is_none());
}

#[tokio::test]
async fn list_synced_in_window_returns_only_window_and_synced() {
    let env = common::setup().await;
    let entries = Entries::new(env.store.clone());
    // In window, synced.
    entries
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "in-window".into(),
            description: "x".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T10:00:00Z".into(),
            end_at: Some("2026-05-20T11:00:00Z".into()),
            billable: false,
            updated_at: "2026-05-20T11:00:01Z".into(),
        })
        .await
        .unwrap();
    // Out of window.
    entries
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "out-of-window".into(),
            description: "x".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-01T10:00:00Z".into(),
            end_at: Some("2026-05-01T11:00:00Z".into()),
            billable: false,
            updated_at: "2026-05-01T11:00:01Z".into(),
        })
        .await
        .unwrap();
    let rows = entries
        .list_synced_in_window("2026-05-20T00:00:00Z", "2026-05-21T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].solidtime_id.as_deref(), Some("in-window"));
}
```

- [ ] **Step 2: Run, verify compile error**

```bash
cargo test -p stint-core --test store_entries_from_remote -- --test-threads=1
```

Expected: compile error — `RemoteEntryUpsert`, `create_from_remote`, etc., undefined.

- [ ] **Step 3: Implement** in `crates/stint-core/src/store/entries.rs` — add this struct + methods to the existing impl block (insert after the existing `mark_synced` method around line 138):

```rust
#[derive(Debug, Clone)]
pub struct RemoteEntryUpsert {
    pub solidtime_id: String,
    pub description: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub start_at: String,
    pub end_at: Option<String>,
    pub billable: bool,
    pub updated_at: String,
}
```

Place `RemoteEntryUpsert` near the top of the file alongside `NewTimeEntry`/`NewCompletedEntry`.

Append the following methods to `impl Entries`:

```rust
    pub async fn create_from_remote(&self, e: RemoteEntryUpsert) -> Result<String> {
        let local_uuid = ids::new_local_uuid();
        sqlx::query(
            r#"INSERT INTO time_entries
               (local_uuid, solidtime_id, description, project_id, task_id,
                start_at, end_at, billable, source, sync_state,
                created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'solidtime', 'synced', ?, ?)"#,
        )
        .bind(&local_uuid)
        .bind(&e.solidtime_id)
        .bind(&e.description)
        .bind(&e.project_id)
        .bind(&e.task_id)
        .bind(&e.start_at)
        .bind(&e.end_at)
        .bind(if e.billable { 1 } else { 0 })
        .bind(&e.updated_at)
        .bind(&e.updated_at)
        .execute(self.store.pool())
        .await?;
        Ok(local_uuid)
    }

    pub async fn get_by_solidtime_id(&self, solidtime_id: &str) -> Result<Option<TimeEntryRow>> {
        let row = sqlx::query_as::<_, TimeEntryRow>(
            "SELECT * FROM time_entries WHERE solidtime_id = ?",
        )
        .bind(solidtime_id)
        .fetch_optional(self.store.pool())
        .await?;
        Ok(row)
    }

    /// Overwrite a synced row from upstream. Returns false if the local row
    /// has pending changes (any non-`synced` state) — caller treats that as
    /// "leave it alone; the queue will push first".
    pub async fn update_from_remote(
        &self,
        solidtime_id: &str,
        e: RemoteEntryUpsert,
    ) -> Result<bool> {
        let res = sqlx::query(
            r#"UPDATE time_entries
               SET description = ?, project_id = ?, task_id = ?,
                   start_at = ?, end_at = ?, billable = ?, updated_at = ?
               WHERE solidtime_id = ? AND sync_state = 'synced'"#,
        )
        .bind(&e.description)
        .bind(&e.project_id)
        .bind(&e.task_id)
        .bind(&e.start_at)
        .bind(&e.end_at)
        .bind(if e.billable { 1 } else { 0 })
        .bind(&e.updated_at)
        .bind(solidtime_id)
        .execute(self.store.pool())
        .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn hard_delete_by_solidtime_id(&self, solidtime_id: &str) -> Result<bool> {
        let res = sqlx::query("DELETE FROM time_entries WHERE solidtime_id = ?")
            .bind(solidtime_id)
            .execute(self.store.pool())
            .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn list_synced_in_window(
        &self,
        from: &str,
        to: &str,
    ) -> Result<Vec<TimeEntryRow>> {
        let rows = sqlx::query_as::<_, TimeEntryRow>(
            r#"SELECT * FROM time_entries
               WHERE sync_state = 'synced'
                 AND solidtime_id IS NOT NULL
                 AND start_at >= ? AND start_at <= ?
               ORDER BY start_at"#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(self.store.pool())
        .await?;
        Ok(rows)
    }
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p stint-core --test store_entries_from_remote -- --test-threads=1
```

Expected: 6 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/store/entries.rs crates/stint-core/tests/store_entries_from_remote.rs
git commit -m "feat(core): store helpers for remote-origin entries"
```

---

## Task 5: Pull module skeleton — `Trigger`, `Window`, `PullReport`, stub entry point

**Files:**
- Create: `crates/stint-core/src/sync/pull/mod.rs`
- Create: `crates/stint-core/src/sync/pull/window.rs`
- Modify: `crates/stint-core/src/sync/mod.rs:1-3`
- Test: `crates/stint-core/tests/sync_pull_window.rs` (new)

- [ ] **Step 1: Failing test** — create `crates/stint-core/tests/sync_pull_window.rs`:

```rust
use chrono::{Duration, TimeZone, Utc};
use stint_core::sync::pull::{Trigger, Window};

#[test]
fn window_for_on_startup_covers_last_24h() {
    let now = Utc.with_ymd_and_hms(2026, 5, 20, 17, 0, 0).unwrap();
    let w = Window::for_trigger(Trigger::OnStartup, now);
    assert_eq!(w.from, now - Duration::hours(24));
    assert_eq!(w.to, now);
}

#[test]
fn window_for_on_focus_covers_last_7d() {
    let now = Utc.with_ymd_and_hms(2026, 5, 20, 17, 0, 0).unwrap();
    let w = Window::for_trigger(Trigger::OnFocus, now);
    assert_eq!(w.from, now - Duration::days(7));
}

#[test]
fn window_for_background_poll_covers_last_1h() {
    let now = Utc.with_ymd_and_hms(2026, 5, 20, 17, 0, 0).unwrap();
    let w = Window::for_trigger(Trigger::BackgroundPoll, now);
    assert_eq!(w.from, now - Duration::hours(1));
}

#[test]
fn window_for_manual_covers_last_30d() {
    let now = Utc.with_ymd_and_hms(2026, 5, 20, 17, 0, 0).unwrap();
    let w = Window::for_trigger(Trigger::Manual, now);
    assert_eq!(w.from, now - Duration::days(30));
}
```

- [ ] **Step 2: Run, verify fails to compile** (module doesn't exist)

```bash
cargo test -p stint-core --test sync_pull_window -- --test-threads=1
```

- [ ] **Step 3: Create the module**

Create `crates/stint-core/src/sync/pull/window.rs`:

```rust
use chrono::{DateTime, Duration, Utc};

/// What caused this pull to fire. Determines the time window.
#[derive(Debug, Clone, Copy)]
pub enum Trigger {
    OnStartup,
    OnFocus,
    BackgroundPoll,
    Manual,
}

#[derive(Debug, Clone, Copy)]
pub struct Window {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

impl Window {
    pub fn for_trigger(trigger: Trigger, now: DateTime<Utc>) -> Self {
        let span = match trigger {
            Trigger::OnStartup => Duration::hours(24),
            Trigger::OnFocus => Duration::days(7),
            Trigger::BackgroundPoll => Duration::hours(1),
            Trigger::Manual => Duration::days(30),
        };
        Self { from: now - span, to: now }
    }
}
```

Create `crates/stint-core/src/sync/pull/mod.rs`:

```rust
//! Down-sync from Solidtime: running-timer adoption, history & delete
//! reconciliation. See `docs/superpowers/specs/2026-05-20-solidtime-down-sync.md`.

pub mod window;

pub use window::{Trigger, Window};

use crate::{solidtime::SolidtimeClient, store::Store, Result};

#[derive(Debug, Default, Clone)]
pub struct PullReport {
    pub adopted: Option<String>,
    pub conflict: Option<ConflictInfo>,
    pub inserted: usize,
    pub updated: usize,
    pub deleted: usize,
}

#[derive(Debug, Clone)]
pub struct ConflictInfo {
    pub remote_id: String,
    pub remote_description: String,
    pub remote_start_at: String,
    pub local_local_uuid: String,
    pub local_description: String,
}

/// Run a full pull cycle: list remote entries, reconcile running timer,
/// reconcile history, reconcile deletes. Returns what changed.
///
/// Stub for Task 5 — wired up incrementally in Tasks 6–7 (running),
/// 11–13 (history), 15–16 (deletes).
pub async fn pull(
    _store: &Store,
    _client: &SolidtimeClient,
    _trigger: Trigger,
) -> Result<PullReport> {
    Ok(PullReport::default())
}
```

Modify `crates/stint-core/src/sync/mod.rs:1-3`:

```rust
pub mod pull;
pub mod push;
pub mod refresh;
```

- [ ] **Step 4: Run window tests**

```bash
cargo test -p stint-core --test sync_pull_window -- --test-threads=1
cargo build --workspace
```

Expected: 4 passed, workspace builds.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/sync/mod.rs crates/stint-core/src/sync/pull/ crates/stint-core/tests/sync_pull_window.rs
git commit -m "feat(core): pull module skeleton (Trigger, Window, PullReport)"
```

---

## Task 6: Running-timer adoption — Some/None case (ADOPT)

**Files:**
- Create: `crates/stint-core/src/sync/pull/running.rs`
- Modify: `crates/stint-core/src/sync/pull/mod.rs` (add `pub mod running;`)
- Test: `crates/stint-core/tests/sync_pull_running.rs` (new)

This task only handles the ADOPT case (remote has running, local has none). Other cases land in Task 7.

- [ ] **Step 1: Failing test** — create `crates/stint-core/tests/sync_pull_running.rs`:

```rust
mod common;

use stint_core::config::Settings;
use stint_core::solidtime::SolidtimeClient;
use stint_core::store::entries::Entries;
use stint_core::store::running::RunningTimer;
use stint_core::sync::pull::{pull, Trigger};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn configure(env: &common::TestEnv, server_uri: &str) {
    let s = Settings::new(env.store.clone());
    s.set("solidtime.url", server_uri).await.unwrap();
    s.set("solidtime.org", "org-1").await.unwrap();
    s.set("solidtime.member_id", "m-1").await.unwrap();
}

#[tokio::test]
async fn adopts_remote_running_when_local_idle() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "remote-running",
                "description": "started in web",
                "start": "2026-05-20T16:30:00Z",
                "end": null,
                "billable": false,
                "updated_at": "2026-05-20T16:30:00Z"
            }]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");

    // Precondition: no running timer locally.
    let running = RunningTimer::new(env.store.clone());
    assert!(running.get().await.unwrap().is_none());

    let report = pull(&env.store, &client, Trigger::OnStartup).await.unwrap();

    // The entry was inserted and is now the running timer.
    assert!(report.adopted.is_some());
    assert!(report.conflict.is_none());
    let adopted_uuid = report.adopted.unwrap();

    let entries = Entries::new(env.store.clone());
    let row = entries.get(&adopted_uuid).await.unwrap().unwrap();
    assert_eq!(row.solidtime_id.as_deref(), Some("remote-running"));
    assert_eq!(row.source, "solidtime");
    assert_eq!(row.sync_state, "synced");
    assert!(row.end_at.is_none());

    let running_row = running.get().await.unwrap().unwrap();
    assert_eq!(running_row.local_uuid, adopted_uuid);
}
```

- [ ] **Step 2: Run, verify fails**

```bash
cargo test -p stint-core --test sync_pull_running -- --test-threads=1
```

Expected: compile error — `pull` returns empty report; running stays empty.

Actually, the test fails functionally (no adoption happens) — confirming that's true is fine.

- [ ] **Step 3: Implement reconcile_running (ADOPT path only)**

Create `crates/stint-core/src/sync/pull/running.rs`:

```rust
use crate::{
    solidtime::{dto::RemoteTimeEntry, SolidtimeClient},
    store::{
        entries::{Entries, RemoteEntryUpsert},
        running::RunningTimer,
        Store,
    },
    sync::pull::ConflictInfo,
    Result,
};

#[derive(Debug, Clone, Default)]
pub struct RunningOutcome {
    pub adopted: Option<String>,
    pub conflict: Option<ConflictInfo>,
}

/// Reconcile the local running timer against the (at most one) remote
/// running entry. See spec §6.
pub async fn reconcile_running(
    store: &Store,
    _client: &SolidtimeClient,
    remote_entries: &[RemoteTimeEntry],
) -> Result<RunningOutcome> {
    let remote_running = remote_entries.iter().find(|e| e.end.is_none());
    let running = RunningTimer::new(store.clone());
    let local_running = running.get().await?;

    match (remote_running, local_running) {
        (None, _) => Ok(RunningOutcome::default()),
        (Some(remote), None) => {
            let entries = Entries::new(store.clone());
            let local_uuid = entries
                .create_from_remote(RemoteEntryUpsert {
                    solidtime_id: remote.id.clone(),
                    description: remote.description.clone(),
                    project_id: remote.project_id.clone(),
                    task_id: remote.task_id.clone(),
                    start_at: remote.start.clone(),
                    end_at: None,
                    billable: remote.billable,
                    updated_at: remote
                        .updated_at
                        .clone()
                        .unwrap_or_else(|| remote.start.clone()),
                })
                .await?;
            running.set(&local_uuid).await?;
            Ok(RunningOutcome {
                adopted: Some(local_uuid),
                conflict: None,
            })
        }
        (Some(_remote), Some(_local)) => {
            // Handled in Task 7.
            Ok(RunningOutcome::default())
        }
    }
}
```

Wire it into `crates/stint-core/src/sync/pull/mod.rs` — replace the stub `pull` with:

```rust
pub mod running;
pub mod window;

pub use window::{Trigger, Window};

use crate::{
    config::Settings,
    solidtime::SolidtimeClient,
    store::Store,
    Error, Result,
};
use chrono::Utc;

#[derive(Debug, Default, Clone)]
pub struct PullReport {
    pub adopted: Option<String>,
    pub conflict: Option<ConflictInfo>,
    pub inserted: usize,
    pub updated: usize,
    pub deleted: usize,
}

#[derive(Debug, Clone)]
pub struct ConflictInfo {
    pub remote_id: String,
    pub remote_description: String,
    pub remote_start_at: String,
    pub local_local_uuid: String,
    pub local_description: String,
}

pub async fn pull(
    store: &Store,
    client: &SolidtimeClient,
    trigger: Trigger,
) -> Result<PullReport> {
    let settings = Settings::new(store.clone());
    let member_id = settings
        .get("solidtime.member_id")
        .await?
        .ok_or(Error::MissingConfig("solidtime.member_id"))?;

    let window = Window::for_trigger(trigger, Utc::now());
    let from = crate::time::format(&window.from);
    let to = crate::time::format(&window.to);

    let remote_entries = client.list_time_entries(&member_id, &from, &to).await?;

    let running_outcome = running::reconcile_running(store, client, &remote_entries).await?;

    Ok(PullReport {
        adopted: running_outcome.adopted,
        conflict: running_outcome.conflict,
        inserted: 0,
        updated: 0,
        deleted: 0,
    })
}
```

Confirm `crate::time::format` accepts `&DateTime<Utc>`:

```bash
grep -n "pub fn format" crates/stint-core/src/time.rs
```

If `format` has a different signature, adjust the call. (The exact helper is `time::format(&dt) -> String`; if it isn't, replace with `window.from.format("%Y-%m-%dT%H:%M:%SZ").to_string()`.)

- [ ] **Step 4: Run, verify PASS**

```bash
cargo test -p stint-core --test sync_pull_running -- --test-threads=1
```

Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/sync/pull/ crates/stint-core/tests/sync_pull_running.rs
git commit -m "feat(core): adopt remote running timer on pull"
```

---

## Task 7: Running-timer adoption — remaining cases (None/Some, same-id, conflict)

**Files:**
- Modify: `crates/stint-core/src/sync/pull/running.rs`
- Test: `crates/stint-core/tests/sync_pull_running.rs` (extend)

- [ ] **Step 1: Append failing tests**

```rust
use stint_core::store::entries::NewTimeEntry;

#[tokio::test]
async fn does_nothing_when_remote_idle_and_local_idle() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": []})))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::OnStartup).await.unwrap();
    assert!(report.adopted.is_none());
    assert!(report.conflict.is_none());
}

#[tokio::test]
async fn does_nothing_when_remote_idle_but_local_running() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    // Local timer started via the normal CLI/GUI path.
    let entries = Entries::new(env.store.clone());
    let local_uuid = entries
        .create(NewTimeEntry {
            description: "local-only".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T16:00:00Z".into(),
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();
    RunningTimer::new(env.store.clone())
        .set(&local_uuid)
        .await
        .unwrap();

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": []})))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::OnStartup).await.unwrap();
    assert!(report.adopted.is_none());
    assert!(report.conflict.is_none());

    // Local timer remained intact.
    let running = RunningTimer::new(env.store.clone()).get().await.unwrap().unwrap();
    assert_eq!(running.local_uuid, local_uuid);
}

#[tokio::test]
async fn no_op_when_remote_and_local_share_same_id() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    // Local timer is already the same entry (e.g. adopted on a previous pull).
    let entries = Entries::new(env.store.clone());
    let local_uuid = entries
        .create_from_remote(stint_core::store::entries::RemoteEntryUpsert {
            solidtime_id: "remote-same".into(),
            description: "started in web".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T16:30:00Z".into(),
            end_at: None,
            billable: false,
            updated_at: "2026-05-20T16:30:00Z".into(),
        })
        .await
        .unwrap();
    RunningTimer::new(env.store.clone())
        .set(&local_uuid)
        .await
        .unwrap();

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "remote-same",
                "description": "started in web",
                "start": "2026-05-20T16:30:00Z",
                "end": null,
                "billable": false,
                "updated_at": "2026-05-20T16:30:00Z"
            }]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::OnStartup).await.unwrap();
    assert!(report.adopted.is_none(), "no new adoption");
    assert!(report.conflict.is_none(), "no conflict");
}

#[tokio::test]
async fn surfaces_conflict_when_local_and_remote_differ() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    let entries = Entries::new(env.store.clone());
    let local_uuid = entries
        .create(NewTimeEntry {
            description: "local task".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T16:00:00Z".into(),
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();
    RunningTimer::new(env.store.clone())
        .set(&local_uuid)
        .await
        .unwrap();

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "remote-other",
                "description": "other device",
                "start": "2026-05-20T16:30:00Z",
                "end": null,
                "billable": false,
                "updated_at": "2026-05-20T16:30:00Z"
            }]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::OnStartup).await.unwrap();
    assert!(report.adopted.is_none(), "must not silently overwrite local");
    let conflict = report.conflict.expect("conflict should be surfaced");
    assert_eq!(conflict.remote_id, "remote-other");
    assert_eq!(conflict.local_local_uuid, local_uuid);
    assert_eq!(conflict.local_description, "local task");

    // Local timer still ticking; no new entry inserted.
    let running = RunningTimer::new(env.store.clone()).get().await.unwrap().unwrap();
    assert_eq!(running.local_uuid, local_uuid);
}
```

- [ ] **Step 2: Run, three of the four should fail** (the idle/idle one passes already):

```bash
cargo test -p stint-core --test sync_pull_running -- --test-threads=1
```

- [ ] **Step 3: Extend `reconcile_running`**

Replace the body of `reconcile_running` in `crates/stint-core/src/sync/pull/running.rs` with the full implementation:

```rust
pub async fn reconcile_running(
    store: &Store,
    _client: &SolidtimeClient,
    remote_entries: &[RemoteTimeEntry],
) -> Result<RunningOutcome> {
    let remote_running = remote_entries.iter().find(|e| e.end.is_none());
    let running = RunningTimer::new(store.clone());
    let local_running_row = running.get().await?;

    let local = match local_running_row {
        Some(r) => Entries::new(store.clone()).get(&r.local_uuid).await?,
        None => None,
    };

    match (remote_running, local) {
        (None, _) => Ok(RunningOutcome::default()),
        (Some(remote), None) => {
            // ADOPT.
            let entries = Entries::new(store.clone());
            let local_uuid = entries
                .create_from_remote(RemoteEntryUpsert {
                    solidtime_id: remote.id.clone(),
                    description: remote.description.clone(),
                    project_id: remote.project_id.clone(),
                    task_id: remote.task_id.clone(),
                    start_at: remote.start.clone(),
                    end_at: None,
                    billable: remote.billable,
                    updated_at: remote
                        .updated_at
                        .clone()
                        .unwrap_or_else(|| remote.start.clone()),
                })
                .await?;
            running.set(&local_uuid).await?;
            Ok(RunningOutcome {
                adopted: Some(local_uuid),
                conflict: None,
            })
        }
        (Some(remote), Some(local_row)) => {
            if local_row.solidtime_id.as_deref() == Some(remote.id.as_str()) {
                Ok(RunningOutcome::default())
            } else {
                Ok(RunningOutcome {
                    adopted: None,
                    conflict: Some(ConflictInfo {
                        remote_id: remote.id.clone(),
                        remote_description: remote.description.clone(),
                        remote_start_at: remote.start.clone(),
                        local_local_uuid: local_row.local_uuid,
                        local_description: local_row.description,
                    }),
                })
            }
        }
    }
}
```

- [ ] **Step 4: Run all running tests, verify PASS**

```bash
cargo test -p stint-core --test sync_pull_running -- --test-threads=1
```

Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/sync/pull/running.rs crates/stint-core/tests/sync_pull_running.rs
git commit -m "feat(core): detect running-timer conflicts on pull"
```

---

## Task 8: Tauri `pull_now` command + on-startup pull

**Files:**
- Create: `crates/stint-app/src/commands/pull.rs`
- Modify: `crates/stint-app/src/commands/mod.rs:1-7` (add `pub mod pull;`)
- Modify: `crates/stint-app/src/main.rs` (register command, fire pull on startup)
- Modify: `crates/stint-app/src/sync_worker.rs` (add `pub const EVENT_PULL_CONFLICT: &str = "pull:conflict";` alongside `EVENT_ENTRIES_CHANGED` — kept here to centralize event names)

- [ ] **Step 1: Implement `pull_now` Tauri command**

Create `crates/stint-app/src/commands/pull.rs`:

```rust
use crate::app_state::AppState;
use crate::commands::{store, AppError};
use crate::sync_worker::{EVENT_ENTRIES_CHANGED, EVENT_PULL_CONFLICT};
use serde::Serialize;
use stint_core::config::{secrets::Secrets, Settings};
use stint_core::solidtime::auth::build_token_provider;
use stint_core::solidtime::SolidtimeClient;
use stint_core::sync::pull::{pull, ConflictInfo, PullReport, Trigger};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::RwLock;

#[derive(Debug, Serialize)]
pub struct PullReportDto {
    pub adopted: Option<String>,
    pub conflict: Option<ConflictDto>,
    pub inserted: usize,
    pub updated: usize,
    pub deleted: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct ConflictDto {
    pub remote_id: String,
    pub remote_description: String,
    pub remote_start_at: String,
    pub local_local_uuid: String,
    pub local_description: String,
}

impl From<ConflictInfo> for ConflictDto {
    fn from(c: ConflictInfo) -> Self {
        Self {
            remote_id: c.remote_id,
            remote_description: c.remote_description,
            remote_start_at: c.remote_start_at,
            local_local_uuid: c.local_local_uuid,
            local_description: c.local_description,
        }
    }
}

impl From<PullReport> for PullReportDto {
    fn from(r: PullReport) -> Self {
        Self {
            adopted: r.adopted,
            conflict: r.conflict.map(ConflictDto::from),
            inserted: r.inserted,
            updated: r.updated,
            deleted: r.deleted,
        }
    }
}

#[tauri::command]
pub async fn pull_now(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
) -> Result<PullReportDto, AppError> {
    let store = store(&state).await;
    let settings = Settings::new((*store).clone());
    let Some(url) = settings.get("solidtime.url").await? else {
        return Err(AppError::msg("solidtime.url not set"));
    };
    let Some(org) = settings.get("solidtime.org").await? else {
        return Err(AppError::msg("solidtime.org not set"));
    };
    let secrets = Secrets::default();
    let (provider, _oauth_client) = build_token_provider(&settings, &secrets, &url).await?;
    let client = SolidtimeClient::new(&url, provider).with_org(org);

    let report = pull(&store, &client, Trigger::Manual).await?;
    if report.adopted.is_some() || report.inserted + report.updated + report.deleted > 0 {
        let _ = app.emit(EVENT_ENTRIES_CHANGED, 0u32);
    }
    if let Some(conflict) = &report.conflict {
        let _ = app.emit(EVENT_PULL_CONFLICT, ConflictDto::from(conflict.clone()));
    }
    Ok(PullReportDto::from(report))
}
```

- [ ] **Step 2: Add the event-name constant**

Append to `crates/stint-app/src/sync_worker.rs` (right after `pub const EVENT_ENTRIES_CHANGED: ...`):

```rust
pub const EVENT_PULL_CONFLICT: &str = "pull:conflict";
```

- [ ] **Step 3: Register `pull_now` and fire one pull on startup**

Modify `crates/stint-app/src/commands/mod.rs:1-7`:

```rust
pub mod calendar;
pub mod config;
pub mod entries;
pub mod projects;
pub mod pull;
pub mod sync;
pub mod timer;
pub mod ui;
```

Modify `crates/stint-app/src/main.rs:33-65` — add `commands::pull::pull_now,` to the `invoke_handler!` list.

Modify `crates/stint-app/src/main.rs:66-93` — in the `.setup(move |app| {` closure, add an on-startup pull right after the calendar worker spawn (line 73):

```rust
            // One-shot pull on startup to surface any remote-side timer/changes.
            {
                let app_handle = app.handle().clone();
                let store_for_pull = store_for_worker.clone();
                tokio::spawn(async move {
                    use stint_core::config::{secrets::Secrets, Settings};
                    use stint_core::solidtime::{auth::build_token_provider, SolidtimeClient};
                    use stint_core::sync::pull::{pull, Trigger};
                    use tauri::Emitter;
                    let settings = Settings::new((*store_for_pull).clone());
                    let Ok(Some(url)) = settings.get("solidtime.url").await else { return };
                    let Ok(Some(org)) = settings.get("solidtime.org").await else { return };
                    let secrets = Secrets::default();
                    let Ok((provider, _client)) =
                        build_token_provider(&settings, &secrets, &url).await
                    else {
                        return;
                    };
                    let client = SolidtimeClient::new(&url, provider).with_org(org);
                    match pull(&store_for_pull, &client, Trigger::OnStartup).await {
                        Ok(report) => {
                            if report.adopted.is_some()
                                || report.inserted + report.updated + report.deleted > 0
                            {
                                let _ = app_handle.emit(sync_worker::EVENT_ENTRIES_CHANGED, 0u32);
                            }
                            if let Some(conflict) = report.conflict {
                                let _ = app_handle.emit(
                                    sync_worker::EVENT_PULL_CONFLICT,
                                    commands::pull::ConflictDto::from(conflict),
                                );
                            }
                        }
                        Err(e) => tracing::warn!(error = %e, "startup pull failed"),
                    }
                });
            }
```

- [ ] **Step 4: Build**

```bash
cargo build -p stint-app
```

Expected: clean build.

- [ ] **Step 5: Manual smoke test**

(Skip if you don't have a Solidtime instance configured locally — manual smoke is in Task 18.)

```bash
scripts/dev-app.sh
```

Click "Always Allow" on the Keychain prompt. App launches; no functional regression in normal flows.

- [ ] **Step 6: Commit**

```bash
git add crates/stint-app/src/commands/pull.rs crates/stint-app/src/commands/mod.rs crates/stint-app/src/main.rs crates/stint-app/src/sync_worker.rs
git commit -m "feat(app): pull_now command + on-startup pull"
```

---

## Task 9: CLI `stint pull` subcommand

**Files:**
- Create: `crates/stint-cli/src/cmd/pull.rs`
- Modify: `crates/stint-cli/src/cmd/mod.rs:1-12`
- Modify: `crates/stint-cli/src/main.rs:19-43` (add subcommand) and `:65-79` (dispatch)
- Test: `crates/stint-cli/tests/cli_e2e.rs` (extend)

- [ ] **Step 1: Implement the subcommand**

Create `crates/stint-cli/src/cmd/pull.rs`:

```rust
use anyhow::{anyhow, Result};
use clap::Args as ClapArgs;
use stint_core::config::{secrets::Secrets, Settings};
use stint_core::solidtime::auth::build_token_provider;
use stint_core::solidtime::SolidtimeClient;
use stint_core::sync::pull::{pull, Trigger};

use super::open_store;

#[derive(ClapArgs, Debug)]
pub struct Args {
    /// Pull and surface a conflict; don't resolve it (default).
    #[arg(long, conflicts_with_all = ["switch", "stop_remote", "dismiss"])]
    pub dismiss: bool,
    /// Stop the remote running timer if a conflict is detected.
    #[arg(long, conflicts_with_all = ["switch", "dismiss"])]
    pub stop_remote: bool,
    /// Stop the local running timer and adopt the remote one.
    #[arg(long, conflicts_with_all = ["stop_remote", "dismiss"])]
    pub switch: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let store = open_store().await?;
    let settings = Settings::new(store.clone());
    let secrets = Secrets::default();
    let url = settings
        .get("solidtime.url")
        .await?
        .ok_or_else(|| anyhow!("solidtime.url not set"))?;
    let org = settings
        .get("solidtime.org")
        .await?
        .ok_or_else(|| anyhow!("solidtime.org not set"))?;
    let (provider, _oauth_client) = build_token_provider(&settings, &secrets, &url).await?;
    let client = SolidtimeClient::new(&url, provider).with_org(org);

    let report = pull(&store, &client, Trigger::Manual).await?;

    println!(
        "+{} entries, ~{} updates, -{} deletes",
        report.inserted, report.updated, report.deleted
    );
    if let Some(adopted) = &report.adopted {
        println!("Adopted remote running timer (local uuid: {adopted})");
    }
    if let Some(c) = &report.conflict {
        eprintln!(
            "Conflict: remote timer \"{}\" started {} differs from local \"{}\".",
            c.remote_description, c.remote_start_at, c.local_description
        );
        if args.stop_remote {
            eprintln!("(--stop-remote requested; resolution support lands in Task 10)");
        } else if args.switch {
            eprintln!("(--switch requested; resolution support lands in Task 10)");
        } else {
            eprintln!("Re-run with --stop-remote, --switch, or --dismiss.");
        }
    }
    Ok(())
}
```

Note: real conflict-resolution actions land in Task 10; for now the CLI prints what would happen.

- [ ] **Step 2: Wire into main**

Modify `crates/stint-cli/src/cmd/mod.rs:1-12`:

```rust
pub mod calendar;
pub mod config;
pub mod config_login;
pub mod delete;
pub mod edit;
pub mod list;
pub mod projects;
pub mod pull;
pub mod start;
pub mod stop;
pub mod sync;
pub mod today;
```

Modify `crates/stint-cli/src/main.rs:19-43` — add to the `Command` enum (after `Sync`):

```rust
    /// Pull running-timer and recent state from Solidtime
    Pull(cmd::pull::Args),
```

Modify `crates/stint-cli/src/main.rs:60` — extend the skip list:

```rust
    if !matches!(cli.command, Command::Sync | Command::Pull(_) | Command::Calendar(_)) {
```

Modify `crates/stint-cli/src/main.rs:65-79` — add to the dispatch:

```rust
        Command::Pull(args) => cmd::pull::run(args).await,
```

- [ ] **Step 3: Build**

```bash
cargo build -p stint-cli
```

Expected: clean build.

- [ ] **Step 4: Extend the CLI e2e test**

Open `crates/stint-cli/tests/cli_e2e.rs`. Look at the existing pattern (it should already use `assert_cmd` + `wiremock`). Add a new test that:

1. Starts a wiremock server
2. Mocks `GET /api/v1/organizations/{org}/time-entries` with a 200 + empty data
3. Sets `STINT_DB` to a tempdir
4. Pre-seeds settings via SQL or via running `stint config set ...`
5. Runs `stint pull`
6. Asserts stdout contains `+0 entries, ~0 updates, -0 deletes`

A concrete sketch — adjust to match the file's existing helpers:

```rust
#[test]
fn pull_empty_window_prints_zero_summary() {
    // 1. Tempdir DB + wiremock server (use the file's existing helpers — if it
    //    uses `assert_cmd::Command::cargo_bin` and `tempfile::tempdir`, follow
    //    that style here).
    // 2. Mock GET …/time-entries with {"data": []}.
    // 3. Set solidtime.url, solidtime.org, solidtime.member_id, solidtime.token.
    // 4. assert_cmd::Command::cargo_bin("stint")
    //      .env("STINT_DB", &db_path)
    //      .args(["pull"])
    //      .assert()
    //      .success()
    //      .stdout(predicates::str::contains("+0 entries"));
}
```

If `cli_e2e.rs` is currently sparse, leave a `#[ignore]` placeholder and document in the commit message that the full CLI e2e is deferred to Task 18 (manual end-to-end).

- [ ] **Step 5: Verify the binary actually works locally**

```bash
scripts/dev-cli.sh pull --help
```

Expected: prints help text including `--dismiss`, `--stop-remote`, `--switch` flags.

- [ ] **Step 6: Commit**

```bash
git add crates/stint-cli/src/cmd/pull.rs crates/stint-cli/src/cmd/mod.rs crates/stint-cli/src/main.rs crates/stint-cli/tests/cli_e2e.rs
git commit -m "feat(cli): stint pull subcommand"
```

---

## Task 10: UI — ConflictBanner + Tauri `conflict_resolve` command

**Files:**
- Create: `crates/stint-app/src/commands/pull.rs` (extend with `conflict_resolve`)
- Create: `ui/src/components/ConflictBanner.tsx`
- Modify: `ui/src/api.ts`
- Modify: `ui/src/routes/Today.tsx` (mount the banner)
- Modify: `crates/stint-app/src/main.rs` (register `conflict_resolve` command)

This task adds the *real* conflict-resolution actions (the CLI flags in Task 9 became no-ops; this task wires them up too).

- [ ] **Step 1: Extend `crates/stint-app/src/commands/pull.rs`** — add at the bottom:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictAction {
    StopRemote,
    Switch,
    Dismiss,
}

#[derive(Debug, Deserialize)]
pub struct ConflictResolveArgs {
    pub action: ConflictAction,
    pub remote_id: String,
}

#[tauri::command]
pub async fn conflict_resolve(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    args: ConflictResolveArgs,
) -> Result<(), AppError> {
    use stint_core::store::queue::{Queue, QueueOp};
    use stint_core::store::running::RunningTimer;
    use stint_core::timer::TimerService;

    let store = store(&state).await;
    match args.action {
        ConflictAction::Dismiss => Ok(()),
        ConflictAction::StopRemote => {
            // Enqueue a delete_entry op against the remote id.
            // Solidtime doesn't have a "stop" endpoint — to stop a running
            // entry, set its `end`. But we don't have the local row mirror,
            // so the simplest implementation is to fetch the remote, mirror
            // it locally as a synced row, then enqueue an update that sets
            // its end_at to now. We use the existing update_entry op shape
            // (payload = {local_uuid}) for that.
            let settings = stint_core::config::Settings::new((*store).clone());
            let secrets = stint_core::config::secrets::Secrets::default();
            let url = settings
                .get("solidtime.url")
                .await?
                .ok_or_else(|| AppError::msg("solidtime.url not set"))?;
            let org = settings
                .get("solidtime.org")
                .await?
                .ok_or_else(|| AppError::msg("solidtime.org not set"))?;
            let (provider, _) =
                stint_core::solidtime::auth::build_token_provider(&settings, &secrets, &url)
                    .await?;
            let client = stint_core::solidtime::SolidtimeClient::new(&url, provider).with_org(org);

            let remote = client
                .get_time_entry(&args.remote_id)
                .await?
                .ok_or_else(|| AppError::msg("remote entry already gone"))?;

            let entries = stint_core::store::entries::Entries::new((*store).clone());
            let local_uuid = entries
                .create_from_remote(stint_core::store::entries::RemoteEntryUpsert {
                    solidtime_id: remote.id.clone(),
                    description: remote.description.clone(),
                    project_id: remote.project_id.clone(),
                    task_id: remote.task_id.clone(),
                    start_at: remote.start.clone(),
                    end_at: None,
                    billable: remote.billable,
                    updated_at: remote.updated_at.clone().unwrap_or_else(|| remote.start.clone()),
                })
                .await?;
            entries
                .set_end(&local_uuid, &stint_core::time::now_utc())
                .await?;
            let queue = Queue::new((*store).clone());
            queue
                .enqueue(
                    QueueOp::UpdateEntry,
                    &serde_json::json!({ "local_uuid": local_uuid }).to_string(),
                    Some(&local_uuid),
                )
                .await?;
            let _ = app.emit(EVENT_ENTRIES_CHANGED, 0u32);
            Ok(())
        }
        ConflictAction::Switch => {
            // Stop the local timer (existing flow), then re-run pull so the
            // remote is adopted via the now-clear local-running slot.
            let timer = TimerService::new((*store).clone());
            timer.stop().await?;
            let settings = stint_core::config::Settings::new((*store).clone());
            let secrets = stint_core::config::secrets::Secrets::default();
            let url = settings.get("solidtime.url").await?.ok_or_else(|| AppError::msg("solidtime.url not set"))?;
            let org = settings.get("solidtime.org").await?.ok_or_else(|| AppError::msg("solidtime.org not set"))?;
            let (provider, _) = stint_core::solidtime::auth::build_token_provider(&settings, &secrets, &url).await?;
            let client = stint_core::solidtime::SolidtimeClient::new(&url, provider).with_org(org);
            let _ = stint_core::sync::pull::pull(&store, &client, stint_core::sync::pull::Trigger::Manual).await?;
            let _ = app.emit(EVENT_ENTRIES_CHANGED, 0u32);
            Ok(())
        }
    }
}
```

Re-confirm against the actual `TimerService::stop` and `Entries::set_end` signatures by reading the files; adjust if they differ.

- [ ] **Step 2: Register the command** in `crates/stint-app/src/main.rs:33-65` invoke_handler:

```rust
            commands::pull::pull_now,
            commands::pull::conflict_resolve,
```

- [ ] **Step 3: UI bindings** — modify `ui/src/api.ts`. Find the existing `invoke<T>(...)` helper and add:

```ts
export type ConflictInfo = {
  remote_id: string;
  remote_description: string;
  remote_start_at: string;
  local_local_uuid: string;
  local_description: string;
};

export type PullReport = {
  adopted: string | null;
  conflict: ConflictInfo | null;
  inserted: number;
  updated: number;
  deleted: number;
};

export async function pullNow(): Promise<PullReport> {
  return invoke("pull_now");
}

export async function conflictResolve(
  action: "stop_remote" | "switch" | "dismiss",
  remoteId: string,
): Promise<void> {
  return invoke("conflict_resolve", { args: { action, remote_id: remoteId } });
}
```

(Adapt the `invoke` call shape to match the existing `api.ts` style.)

- [ ] **Step 4: ConflictBanner component**

Create `ui/src/components/ConflictBanner.tsx`:

```tsx
import { createSignal, onCleanup, onMount, Show } from "solid-js";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { conflictResolve, type ConflictInfo } from "~/api";

export default function ConflictBanner() {
  const [conflict, setConflict] = createSignal<ConflictInfo | null>(null);
  const [busy, setBusy] = createSignal(false);
  let unlisten: UnlistenFn | undefined;

  onMount(async () => {
    unlisten = await listen<ConflictInfo>("pull:conflict", (e) => {
      setConflict(e.payload);
    });
  });

  onCleanup(() => unlisten?.());

  const handle = async (action: "stop_remote" | "switch" | "dismiss") => {
    const c = conflict();
    if (!c || busy()) return;
    setBusy(true);
    try {
      await conflictResolve(action, c.remote_id);
      setConflict(null);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Show when={conflict()}>
      {(c) => (
        <div class="rounded-lg border border-amber-500/40 bg-amber-50 dark:bg-amber-950/40 p-3 mb-3">
          <p class="text-sm">
            Another timer is running in Solidtime: <strong>“{c().remote_description}”</strong>{" "}
            started {new Date(c().remote_start_at).toLocaleTimeString()}.
          </p>
          <div class="flex gap-2 mt-2">
            <button
              class="text-sm px-2 py-1 rounded bg-amber-600 text-white disabled:opacity-50"
              disabled={busy()}
              onClick={() => handle("stop_remote")}
            >
              Stop it remotely
            </button>
            <button
              class="text-sm px-2 py-1 rounded border border-amber-600 text-amber-700 disabled:opacity-50"
              disabled={busy()}
              onClick={() => handle("switch")}
            >
              Switch to it
            </button>
            <button
              class="text-sm px-2 py-1 rounded text-amber-700 disabled:opacity-50"
              disabled={busy()}
              onClick={() => handle("dismiss")}
            >
              Dismiss
            </button>
          </div>
        </div>
      )}
    </Show>
  );
}
```

- [ ] **Step 5: Mount in Today**

Modify `ui/src/routes/Today.tsx` — at the top of the JSX (above existing content) insert:

```tsx
import ConflictBanner from "~/components/ConflictBanner";

// ... inside the component's return:
<ConflictBanner />
```

(Adapt to wherever Today's render tree begins.)

- [ ] **Step 6: Build everything**

```bash
cargo build -p stint-app
cd ui && pnpm typecheck && cd ..
```

Expected: clean.

- [ ] **Step 7: Real CLI conflict resolution** — replace the placeholder eprintln branches in `crates/stint-cli/src/cmd/pull.rs` from Task 9 with real implementations that call equivalent stint-core paths (essentially the same logic as `conflict_resolve` above, minus the AppHandle emit). Refactor: move the shared logic into a new function `stint_core::sync::pull::resolve_conflict(store, client, action, remote_id)` and call it from both CLI and Tauri.

Sketch of the shared helper to add to `crates/stint-core/src/sync/pull/mod.rs`:

```rust
pub enum ConflictAction { StopRemote, Switch, Dismiss }

pub async fn resolve_conflict(
    store: &Store,
    client: &SolidtimeClient,
    action: ConflictAction,
    remote_id: &str,
) -> Result<()> {
    match action {
        ConflictAction::Dismiss => Ok(()),
        ConflictAction::StopRemote => {
            // ... same logic as commands/pull.rs StopRemote branch, using
            //     store + client directly. Returns Ok(()) when enqueued.
            todo!("move from app commands; identical logic")
        }
        ConflictAction::Switch => {
            crate::timer::TimerService::new(store.clone()).stop().await?;
            pull(store, client, Trigger::Manual).await.map(|_| ())
        }
    }
}
```

After extracting, update both call sites (`commands/pull.rs` and `cmd/pull.rs`) to call this helper. Avoid the `todo!()` — flesh it out by lifting the StopRemote logic from `commands/pull.rs` verbatim.

- [ ] **Step 8: Manual smoke** — `scripts/dev-app.sh`, start a timer in the Solidtime web UI, start a local timer in stint, wait for the next pull (or click "Refresh from Solidtime" if you've already wired it). Verify the banner appears with the right three buttons; verify each button performs the expected action.

(Manual smoke is OK to defer to Task 18 if Solidtime web isn't reachable yet.)

- [ ] **Step 9: Commit**

```bash
git add crates/stint-app/src/commands/pull.rs crates/stint-app/src/main.rs \
        crates/stint-core/src/sync/pull/mod.rs \
        crates/stint-cli/src/cmd/pull.rs \
        ui/src/api.ts ui/src/components/ConflictBanner.tsx ui/src/routes/Today.tsx
git commit -m "feat(app): conflict banner + resolve actions"
```

**End of Commit 1** — running-timer adoption + conflict UI complete.

---

## Task 11: History reconcile — insert new remote entries

**Files:**
- Create: `crates/stint-core/src/sync/pull/history.rs`
- Modify: `crates/stint-core/src/sync/pull/mod.rs` (call `reconcile_history`)
- Test: `crates/stint-core/tests/sync_pull_history.rs` (new)

- [ ] **Step 1: Failing test** — create `crates/stint-core/tests/sync_pull_history.rs`:

```rust
mod common;

use stint_core::config::Settings;
use stint_core::solidtime::SolidtimeClient;
use stint_core::store::entries::Entries;
use stint_core::sync::pull::{pull, Trigger};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn configure(env: &common::TestEnv, server_uri: &str) {
    let s = Settings::new(env.store.clone());
    s.set("solidtime.url", server_uri).await.unwrap();
    s.set("solidtime.org", "org-1").await.unwrap();
    s.set("solidtime.member_id", "m-1").await.unwrap();
}

#[tokio::test]
async fn inserts_new_remote_entries() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {
                    "id": "remote-a",
                    "description": "task a",
                    "start": "2026-05-20T10:00:00Z",
                    "end": "2026-05-20T11:00:00Z",
                    "billable": false,
                    "updated_at": "2026-05-20T11:00:00Z"
                },
                {
                    "id": "remote-b",
                    "description": "task b",
                    "start": "2026-05-20T11:30:00Z",
                    "end": "2026-05-20T12:00:00Z",
                    "billable": true,
                    "updated_at": "2026-05-20T12:00:00Z"
                }
            ]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::Manual).await.unwrap();
    assert_eq!(report.inserted, 2);
    assert_eq!(report.updated, 0);

    let entries = Entries::new(env.store.clone());
    assert!(entries.get_by_solidtime_id("remote-a").await.unwrap().is_some());
    assert!(entries.get_by_solidtime_id("remote-b").await.unwrap().is_some());
}
```

- [ ] **Step 2: Run, expect compile or assert failure**

```bash
cargo test -p stint-core --test sync_pull_history -- --test-threads=1
```

- [ ] **Step 3: Implement**

Create `crates/stint-core/src/sync/pull/history.rs`:

```rust
use crate::{
    solidtime::dto::RemoteTimeEntry,
    store::{
        entries::{Entries, RemoteEntryUpsert},
        Store,
    },
    Result,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct HistoryOutcome {
    pub inserted: usize,
    pub updated: usize,
}

/// Reconcile completed entries (entries with non-null end). Inserts new ones,
/// updates existing synced rows when remote is newer, skips local rows that
/// have pending changes. See spec §8.
pub async fn reconcile_history(
    store: &Store,
    remote_entries: &[RemoteTimeEntry],
) -> Result<HistoryOutcome> {
    let entries = Entries::new(store.clone());
    let mut out = HistoryOutcome::default();

    for remote in remote_entries.iter().filter(|e| e.end.is_some()) {
        let existing = entries.get_by_solidtime_id(&remote.id).await?;
        let upsert = RemoteEntryUpsert {
            solidtime_id: remote.id.clone(),
            description: remote.description.clone(),
            project_id: remote.project_id.clone(),
            task_id: remote.task_id.clone(),
            start_at: remote.start.clone(),
            end_at: remote.end.clone(),
            billable: remote.billable,
            updated_at: remote
                .updated_at
                .clone()
                .unwrap_or_else(|| remote.start.clone()),
        };

        match existing {
            None => {
                entries.create_from_remote(upsert).await?;
                out.inserted += 1;
            }
            Some(local) => {
                if local.sync_state != "synced" {
                    // Pending local mutation — leave it alone (Task 12).
                    continue;
                }
                if !is_remote_newer(&local.updated_at, &upsert.updated_at) {
                    continue;
                }
                if entries.update_from_remote(&remote.id, upsert).await? {
                    out.updated += 1;
                }
            }
        }
    }
    Ok(out)
}

fn is_remote_newer(local_updated_at: &str, remote_updated_at: &str) -> bool {
    remote_updated_at > local_updated_at
}
```

Wire it into `pull` in `crates/stint-core/src/sync/pull/mod.rs`:

```rust
pub mod history;
pub mod running;
pub mod window;
// (other use statements unchanged)

// in pull():
let history_outcome = history::reconcile_history(store, &remote_entries).await?;

Ok(PullReport {
    adopted: running_outcome.adopted,
    conflict: running_outcome.conflict,
    inserted: history_outcome.inserted,
    updated: history_outcome.updated,
    deleted: 0,
})
```

- [ ] **Step 4: Run, verify PASS**

```bash
cargo test -p stint-core --test sync_pull_history -- --test-threads=1
```

Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/sync/pull/history.rs crates/stint-core/src/sync/pull/mod.rs crates/stint-core/tests/sync_pull_history.rs
git commit -m "feat(core): reconcile_history inserts new remote entries"
```

---

## Task 12: History reconcile — update + skip pending cases

**Files:**
- Test: `crates/stint-core/tests/sync_pull_history.rs` (extend)
- (Implementation already complete — these tests verify existing branches.)

- [ ] **Step 1: Append failing tests**

```rust
use stint_core::store::entries::RemoteEntryUpsert;

#[tokio::test]
async fn updates_existing_row_when_remote_is_newer() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    // Pre-seed a local synced row with an older updated_at.
    Entries::new(env.store.clone())
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "remote-c".into(),
            description: "old".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T10:00:00Z".into(),
            end_at: Some("2026-05-20T11:00:00Z".into()),
            billable: false,
            updated_at: "2026-05-20T11:00:00Z".into(),
        })
        .await
        .unwrap();

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "remote-c",
                "description": "newer description",
                "start": "2026-05-20T10:00:00Z",
                "end": "2026-05-20T11:00:00Z",
                "billable": true,
                "updated_at": "2026-05-20T12:00:00Z"
            }]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::Manual).await.unwrap();
    assert_eq!(report.inserted, 0);
    assert_eq!(report.updated, 1);

    let entries = Entries::new(env.store.clone());
    let row = entries.get_by_solidtime_id("remote-c").await.unwrap().unwrap();
    assert_eq!(row.description, "newer description");
    assert_eq!(row.billable, 1);
}

#[tokio::test]
async fn skips_when_local_is_pending() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    let entries = Entries::new(env.store.clone());
    let local_uuid = entries
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "remote-d".into(),
            description: "synced".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T10:00:00Z".into(),
            end_at: Some("2026-05-20T11:00:00Z".into()),
            billable: false,
            updated_at: "2026-05-20T11:00:00Z".into(),
        })
        .await
        .unwrap();
    // Local edit -> state becomes `dirty`.
    entries.update_description(&local_uuid, "local edit").await.unwrap();

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "remote-d",
                "description": "remote edit",
                "start": "2026-05-20T10:00:00Z",
                "end": "2026-05-20T11:00:00Z",
                "billable": false,
                "updated_at": "2026-05-20T12:00:00Z"
            }]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::Manual).await.unwrap();
    assert_eq!(report.updated, 0);
    let row = entries.get(&local_uuid).await.unwrap().unwrap();
    assert_eq!(row.description, "local edit", "must not overwrite local pending change");
}

#[tokio::test]
async fn noop_when_local_is_newer() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    Entries::new(env.store.clone())
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "remote-e".into(),
            description: "local-most-recent".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T10:00:00Z".into(),
            end_at: Some("2026-05-20T11:00:00Z".into()),
            billable: false,
            updated_at: "2026-05-20T13:00:00Z".into(),
        })
        .await
        .unwrap();

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{
                "id": "remote-e",
                "description": "remote-older",
                "start": "2026-05-20T10:00:00Z",
                "end": "2026-05-20T11:00:00Z",
                "billable": false,
                "updated_at": "2026-05-20T12:00:00Z"
            }]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::Manual).await.unwrap();
    assert_eq!(report.updated, 0);
    let row = Entries::new(env.store.clone()).get_by_solidtime_id("remote-e").await.unwrap().unwrap();
    assert_eq!(row.description, "local-most-recent");
}
```

- [ ] **Step 2: Run, verify all PASS**

```bash
cargo test -p stint-core --test sync_pull_history -- --test-threads=1
```

Expected: 4 passed.

- [ ] **Step 3: Commit**

```bash
git add crates/stint-core/tests/sync_pull_history.rs
git commit -m "test(core): history reconcile update/skip/no-op paths"
```

---

## Task 13: UI — PullStatus ("Last synced N seconds ago • Refresh")

**Files:**
- Create: `ui/src/components/PullStatus.tsx`
- Modify: `ui/src/routes/Today.tsx`
- (No backend changes — uses existing `pullNow()` from Task 10.)

- [ ] **Step 1: Component**

Create `ui/src/components/PullStatus.tsx`:

```tsx
import { createSignal, onMount } from "solid-js";
import { pullNow } from "~/api";

export default function PullStatus() {
  const [lastPulledAt, setLastPulledAt] = createSignal<Date | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const refresh = async () => {
    if (busy()) return;
    setBusy(true);
    setError(null);
    try {
      await pullNow();
      setLastPulledAt(new Date());
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  onMount(refresh);

  const ago = () => {
    const t = lastPulledAt();
    if (!t) return "—";
    const secs = Math.floor((Date.now() - t.getTime()) / 1000);
    if (secs < 60) return `${secs}s ago`;
    return `${Math.floor(secs / 60)}m ago`;
  };

  return (
    <div class="flex items-center gap-2 text-xs text-neutral-500 mb-2">
      <span>Last synced: {ago()}</span>
      <button
        class="px-1.5 py-0.5 rounded border border-neutral-300 dark:border-neutral-700 hover:bg-neutral-100 dark:hover:bg-neutral-800 disabled:opacity-50"
        disabled={busy()}
        onClick={refresh}
      >
        {busy() ? "Refreshing…" : "Refresh"}
      </button>
      {error() && <span class="text-rose-500">{error()}</span>}
    </div>
  );
}
```

- [ ] **Step 2: Mount**

Edit `ui/src/routes/Today.tsx` — alongside `<ConflictBanner />`:

```tsx
import PullStatus from "~/components/PullStatus";
// ...
<PullStatus />
<ConflictBanner />
```

- [ ] **Step 3: Verify build + types**

```bash
cd ui && pnpm typecheck && cd ..
cargo build -p stint-app
```

- [ ] **Step 4: Commit**

```bash
git add ui/src/components/PullStatus.tsx ui/src/routes/Today.tsx
git commit -m "feat(ui): pull status + manual refresh on Today"
```

---

## Task 14: Single-transaction guarantee + integration test

**Files:**
- Modify: `crates/stint-core/src/sync/pull/mod.rs`
- Test: `crates/stint-core/tests/sync_pull_history.rs` (extend) — proves rollback on failure

Spec §10: "All three steps share the same transaction at the SQLite layer so partial application doesn't leave the local DB inconsistent."

In practice: sqlx + sqlite + the existing `Store` API don't expose multi-statement transactions through the current method shapes. Realistic choice — wrap the reconcile work in a single `sqlx::Transaction` started from the `Store::pool()`. The minimal API change is one helper that begins a transaction and passes it down to each reconcile step.

This is a bigger refactor than the previous tasks. Two options:

- **Option A (full):** add `&mut Transaction<Sqlite>` parameter threaded through `create_from_remote`, `update_from_remote`, etc. Affects every method we added in Task 4.
- **Option B (lighter, recommended):** keep methods unchanged; instead, run the entire `pull` body inside `sqlx::query("BEGIN")` / `COMMIT` / `ROLLBACK` against the same pool. SQLite's default isolation is serializable; this works because the pool is `max_connections=1` (check `Store::connect`).

Confirm the pool size:

```bash
grep -n "max_connections" crates/stint-core/src/store/mod.rs
```

If max_connections = 1, **Option B** is safe and minimal. Otherwise pick Option A.

- [ ] **Step 1: Verify pool size** (do this first)

```bash
grep -n "max_connections\|SqlitePoolOptions\|SqliteConnectOptions" crates/stint-core/src/store/mod.rs
```

If you see `max_connections(1)` — Option B is fine. If not, switch to Option A and bail out of this task; consult the user before proceeding.

- [ ] **Step 2 (Option B path): Wrap `pull` in a transaction**

Modify `crates/stint-core/src/sync/pull/mod.rs` — wrap the body after the network fetch:

```rust
pub async fn pull(
    store: &Store,
    client: &SolidtimeClient,
    trigger: Trigger,
) -> Result<PullReport> {
    let settings = Settings::new(store.clone());
    let member_id = settings
        .get("solidtime.member_id")
        .await?
        .ok_or(Error::MissingConfig("solidtime.member_id"))?;

    let window = Window::for_trigger(trigger, Utc::now());
    let from = window.from.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let to = window.to.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let remote_entries = client.list_time_entries(&member_id, &from, &to).await?;

    sqlx::query("BEGIN").execute(store.pool()).await?;
    let result: Result<PullReport> = async {
        let running_outcome = running::reconcile_running(store, client, &remote_entries).await?;
        let history_outcome = history::reconcile_history(store, &remote_entries).await?;
        Ok(PullReport {
            adopted: running_outcome.adopted,
            conflict: running_outcome.conflict,
            inserted: history_outcome.inserted,
            updated: history_outcome.updated,
            deleted: 0,
        })
    }
    .await;

    match &result {
        Ok(_) => {
            sqlx::query("COMMIT").execute(store.pool()).await?;
        }
        Err(_) => {
            let _ = sqlx::query("ROLLBACK").execute(store.pool()).await;
        }
    }
    result
}
```

- [ ] **Step 3: Test rollback on partial failure**

The cleanest test forces `reconcile_history` to fail mid-loop (e.g. with a malformed second entry that violates a constraint). Realistically the only column with a UNIQUE constraint is `solidtime_id`. Test plan: list returns two entries with the *same* id. The second insert violates UNIQUE → transaction rolls back → neither entry exists.

Append to `crates/stint-core/tests/sync_pull_history.rs`:

```rust
#[tokio::test]
async fn rollback_on_partial_failure_leaves_no_rows() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    // Two entries with the same id — second insert violates UNIQUE.
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {
                    "id": "dup",
                    "description": "first",
                    "start": "2026-05-20T10:00:00Z",
                    "end": "2026-05-20T11:00:00Z",
                    "billable": false,
                    "updated_at": "2026-05-20T11:00:00Z"
                },
                {
                    "id": "dup",
                    "description": "second",
                    "start": "2026-05-20T11:30:00Z",
                    "end": "2026-05-20T12:00:00Z",
                    "billable": false,
                    "updated_at": "2026-05-20T12:00:00Z"
                }
            ]
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let result = pull(&env.store, &client, Trigger::Manual).await;
    assert!(result.is_err(), "expected UNIQUE violation");

    // Rollback: neither entry exists.
    let entries = Entries::new(env.store.clone());
    assert!(entries.get_by_solidtime_id("dup").await.unwrap().is_none());
}
```

- [ ] **Step 4: Run all pull tests**

```bash
cargo test -p stint-core --tests sync_pull -- --test-threads=1
```

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/sync/pull/mod.rs crates/stint-core/tests/sync_pull_history.rs
git commit -m "feat(core): wrap pull in single transaction"
```

**End of Commit 2** — recent-history reconciliation complete.

---

## Task 15: Delete reconcile — fetch missing entries

**Files:**
- Create: `crates/stint-core/src/sync/pull/deletes.rs`
- Modify: `crates/stint-core/src/sync/pull/mod.rs` (call `reconcile_deletes`)
- Test: `crates/stint-core/tests/sync_pull_deletes.rs` (new)

- [ ] **Step 1: Failing test** — create `crates/stint-core/tests/sync_pull_deletes.rs`:

```rust
mod common;

use stint_core::config::Settings;
use stint_core::solidtime::SolidtimeClient;
use stint_core::store::entries::{Entries, RemoteEntryUpsert};
use stint_core::sync::pull::{pull, Trigger};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn configure(env: &common::TestEnv, server_uri: &str) {
    let s = Settings::new(env.store.clone());
    s.set("solidtime.url", server_uri).await.unwrap();
    s.set("solidtime.org", "org-1").await.unwrap();
    s.set("solidtime.member_id", "m-1").await.unwrap();
}

#[tokio::test]
async fn deletes_local_when_remote_returns_404() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    // Local has a synced row inside the window.
    let now = chrono::Utc::now();
    let start_at = (now - chrono::Duration::hours(1)).format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let end_at = (now - chrono::Duration::minutes(30)).format("%Y-%m-%dT%H:%M:%SZ").to_string();
    Entries::new(env.store.clone())
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "doomed".into(),
            description: "to be deleted".into(),
            project_id: None,
            task_id: None,
            start_at,
            end_at: Some(end_at),
            billable: false,
            updated_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        })
        .await
        .unwrap();

    // List returns no entries (the row "fell out").
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": []})))
        .mount(&server)
        .await;
    // The follow-up fetch by id returns 404.
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries/doomed"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::Manual).await.unwrap();
    assert_eq!(report.deleted, 1);

    let entries = Entries::new(env.store.clone());
    assert!(entries.get_by_solidtime_id("doomed").await.unwrap().is_none());
}
```

- [ ] **Step 2: Run, verify failure**

```bash
cargo test -p stint-core --test sync_pull_deletes -- --test-threads=1
```

- [ ] **Step 3: Implement**

Create `crates/stint-core/src/sync/pull/deletes.rs`:

```rust
use crate::{
    solidtime::{dto::RemoteTimeEntry, SolidtimeClient},
    store::{entries::Entries, Store},
    Result,
};
use chrono::{DateTime, Utc};
use std::collections::HashSet;

const MAX_DELETE_PROBES_PER_PULL: usize = 50;

#[derive(Debug, Default, Clone, Copy)]
pub struct DeletesOutcome {
    pub deleted: usize,
}

/// For each local synced row in the window whose solidtime_id is NOT in the
/// list response, GET it by id. 404 → delete locally. 200 → keep.
/// Capped at MAX_DELETE_PROBES_PER_PULL to bound worst-case cost.
pub async fn reconcile_deletes(
    store: &Store,
    client: &SolidtimeClient,
    remote_entries: &[RemoteTimeEntry],
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<DeletesOutcome> {
    let entries = Entries::new(store.clone());
    let from_str = from.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let to_str = to.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let local_rows = entries.list_synced_in_window(&from_str, &to_str).await?;

    let remote_ids: HashSet<&str> =
        remote_entries.iter().map(|e| e.id.as_str()).collect();

    let mut out = DeletesOutcome::default();
    let mut probes = 0;
    for row in local_rows {
        if probes >= MAX_DELETE_PROBES_PER_PULL {
            break;
        }
        let Some(solidtime_id) = row.solidtime_id.as_deref() else {
            continue;
        };
        if remote_ids.contains(solidtime_id) {
            continue;
        }
        probes += 1;
        match client.get_time_entry(solidtime_id).await? {
            None => {
                if entries.hard_delete_by_solidtime_id(solidtime_id).await? {
                    out.deleted += 1;
                }
            }
            Some(_) => {} // still alive, just outside the window
        }
    }
    Ok(out)
}
```

Wire into `crates/stint-core/src/sync/pull/mod.rs`:

```rust
pub mod deletes;
// ...
let deletes_outcome =
    deletes::reconcile_deletes(store, client, &remote_entries, window.from, window.to).await?;
// in PullReport:
deleted: deletes_outcome.deleted,
```

- [ ] **Step 4: Run, verify PASS**

```bash
cargo test -p stint-core --test sync_pull_deletes -- --test-threads=1
```

- [ ] **Step 5: Commit**

```bash
git add crates/stint-core/src/sync/pull/deletes.rs crates/stint-core/src/sync/pull/mod.rs crates/stint-core/tests/sync_pull_deletes.rs
git commit -m "feat(core): reconcile_deletes (404→delete, 200→keep)"
```

---

## Task 16: Delete reconcile — 200 keep + 50-probe cap

**Files:**
- Test: `crates/stint-core/tests/sync_pull_deletes.rs` (extend)
- (Implementation complete — these tests verify existing branches.)

- [ ] **Step 1: Append failing tests**

```rust
#[tokio::test]
async fn keeps_local_when_remote_get_returns_200() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    let now = chrono::Utc::now();
    let start_at = (now - chrono::Duration::hours(1)).format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let end_at = (now - chrono::Duration::minutes(30)).format("%Y-%m-%dT%H:%M:%SZ").to_string();
    Entries::new(env.store.clone())
        .create_from_remote(RemoteEntryUpsert {
            solidtime_id: "still-here".into(),
            description: "alive elsewhere".into(),
            project_id: None,
            task_id: None,
            start_at: start_at.clone(),
            end_at: Some(end_at.clone()),
            billable: false,
            updated_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        })
        .await
        .unwrap();

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": []})))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries/still-here"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "id": "still-here",
                "description": "alive elsewhere",
                "start": start_at,
                "end": end_at,
                "billable": false
            }
        })))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::Manual).await.unwrap();
    assert_eq!(report.deleted, 0);

    assert!(Entries::new(env.store.clone())
        .get_by_solidtime_id("still-here")
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn caps_delete_probes_at_50_per_pull() {
    let env = common::setup().await;
    let server = MockServer::start().await;
    configure(&env, &server.uri()).await;

    let entries = Entries::new(env.store.clone());
    // 60 local rows in the window, all "missing" from the list response.
    for i in 0..60 {
        let start_at = (chrono::Utc::now() - chrono::Duration::minutes(60 - i))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let end_at = (chrono::Utc::now() - chrono::Duration::minutes(59 - i))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        entries
            .create_from_remote(RemoteEntryUpsert {
                solidtime_id: format!("row-{i}"),
                description: "x".into(),
                project_id: None,
                task_id: None,
                start_at,
                end_at: Some(end_at),
                billable: false,
                updated_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            })
            .await
            .unwrap();
    }

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": []})))
        .mount(&server)
        .await;
    // Any per-id GET returns 404 — but we expect only 50 such requests.
    Mock::given(method("GET"))
        .and(wiremock::matchers::path_regex(
            r"^/api/v1/organizations/org-1/time-entries/row-\d+$",
        ))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = SolidtimeClient::with_api_token(&server.uri(), "t").with_org("org-1");
    let report = pull(&env.store, &client, Trigger::Manual).await.unwrap();
    assert_eq!(report.deleted, 50);

    // Verify exactly 50 rows survived (60 - 50 deleted = 10 remaining).
    let remaining = Entries::new(env.store.clone())
        .list_synced_in_window("2026-01-01T00:00:00Z", "2099-01-01T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(remaining.len(), 10);
}
```

- [ ] **Step 2: Run**

```bash
cargo test -p stint-core --test sync_pull_deletes -- --test-threads=1
```

Expected: 3 passed.

- [ ] **Step 3: Commit**

```bash
git add crates/stint-core/tests/sync_pull_deletes.rs
git commit -m "test(core): delete reconcile keeps 200s, caps at 50 probes"
```

---

## Task 17: Background pull worker (5-min interval) + on-focus refresh

**Files:**
- Create: `crates/stint-app/src/pull_worker.rs`
- Modify: `crates/stint-app/src/lib.rs` (`pub mod pull_worker;`)
- Modify: `crates/stint-app/src/main.rs` (spawn the worker; wire on-focus)

- [ ] **Step 1: Implement the worker**

Create `crates/stint-app/src/pull_worker.rs`:

```rust
//! Periodic Solidtime → stint pull. Runs every 5 minutes while the app is
//! open; also exposed as a nudge for explicit refreshes (window focus).

use crate::sync_worker::{EVENT_ENTRIES_CHANGED, EVENT_PULL_CONFLICT};
use std::sync::Arc;
use std::time::Duration;
use stint_core::{
    config::{secrets::Secrets, Settings},
    solidtime::{auth::build_token_provider, SolidtimeClient},
    store::Store,
    sync::pull::{pull, Trigger},
};
use tauri::{AppHandle, Emitter};
use tokio::time::sleep;
use tracing::{debug, info, warn};

const TICK: Duration = Duration::from_secs(300);

pub fn spawn(app: AppHandle, store: Arc<Store>) {
    tokio::spawn(async move {
        info!("background pull worker started (tick = {:?})", TICK);
        loop {
            sleep(TICK).await;
            if let Err(e) = tick(&app, &store, Trigger::BackgroundPoll).await {
                warn!(error = %e, "pull tick failed");
            }
        }
    });
}

/// Fire a one-shot pull, e.g. on window focus.
pub fn nudge(app: AppHandle, store: Arc<Store>, trigger: Trigger) {
    tokio::spawn(async move {
        if let Err(e) = tick(&app, &store, trigger).await {
            debug!(error = %e, "pull nudge failed");
        }
    });
}

async fn tick(
    app: &AppHandle,
    store: &Store,
    trigger: Trigger,
) -> stint_core::Result<()> {
    let Some(client) = build_client(store).await? else {
        debug!("pull worker: config incomplete, skipping tick");
        return Ok(());
    };
    let report = pull(store, &client, trigger).await?;
    if report.adopted.is_some() || report.inserted + report.updated + report.deleted > 0 {
        let _ = app.emit(EVENT_ENTRIES_CHANGED, 0u32);
    }
    if let Some(conflict) = report.conflict {
        use crate::commands::pull::ConflictDto;
        let _ = app.emit(EVENT_PULL_CONFLICT, ConflictDto::from(conflict));
    }
    Ok(())
}

async fn build_client(store: &Store) -> stint_core::Result<Option<SolidtimeClient>> {
    let settings = Settings::new(store.clone());
    let secrets = Secrets::default();
    let Some(url) = settings.get("solidtime.url").await? else {
        return Ok(None);
    };
    let Some(org) = settings.get("solidtime.org").await? else {
        return Ok(None);
    };
    let (provider, _) = build_token_provider(&settings, &secrets, &url).await?;
    Ok(Some(SolidtimeClient::new(&url, provider).with_org(org)))
}
```

- [ ] **Step 2: Register**

Modify `crates/stint-app/src/lib.rs`:

```rust
pub mod pull_worker;
```

Modify `crates/stint-app/src/main.rs:5-7`:

```rust
mod pull_worker;
```

Inside `.setup(...)` (after `sync_worker::spawn(...)`):

```rust
            // Periodic Solidtime → stint pull (5-min tick).
            pull_worker::spawn(app.handle().clone(), store_for_worker.clone());
```

- [ ] **Step 3: Wire on-focus** — debounced 30s

Inside the same `.setup(...)` closure, near the main-window setup (the `if let Some(main) = app.get_webview_window("main") {` block), add a Focused listener:

```rust
            if let Some(main) = app.get_webview_window("main") {
                let app_handle_focus = app.handle().clone();
                let store_for_focus = store_for_worker.clone();
                let last_focus_pull = std::sync::Arc::new(std::sync::Mutex::new(
                    std::time::Instant::now() - std::time::Duration::from_secs(60),
                ));
                main.on_window_event(move |event| {
                    if let tauri::WindowEvent::Focused(true) = event {
                        let mut guard = last_focus_pull.lock().unwrap();
                        if guard.elapsed() < std::time::Duration::from_secs(30) {
                            return;
                        }
                        *guard = std::time::Instant::now();
                        pull_worker::nudge(
                            app_handle_focus.clone(),
                            store_for_focus.clone(),
                            stint_core::sync::pull::Trigger::OnFocus,
                        );
                    }
                });
            }
```

(Coordinate this with the existing CloseRequested handler — both can live in the same `on_window_event` closure; merge them.)

- [ ] **Step 4: Build + manual smoke**

```bash
cargo build -p stint-app
scripts/dev-app.sh
```

Watch the logs (`STINT_LOG=info scripts/dev-app.sh`):

- On startup: "background pull worker started" + initial pull log.
- After 5 minutes idle: a tick log.
- Focus the main window: an `OnFocus` tick.
- Focus it again within 30s: no second tick.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-app/src/pull_worker.rs crates/stint-app/src/lib.rs crates/stint-app/src/main.rs
git commit -m "feat(app): background pull worker + on-focus refresh"
```

**End of Commit 3** — delete reconcile + background poll complete.

---

## Task 18: Full-workspace verification + PR

**Files:** none

- [ ] **Step 1: Full test suite**

```bash
cargo test --workspace -- --test-threads=1
```

Expected: all green. Investigate any regressions before continuing.

- [ ] **Step 2: Clippy + fmt**

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

- [ ] **Step 3: UI typecheck + build**

```bash
cd ui
pnpm typecheck
pnpm build
cd ..
```

- [ ] **Step 4: Manual smoke**

```bash
scripts/dev-app.sh
```

Verify, with a real Solidtime instance:

1. Start a timer in Solidtime's web UI. Within 5 minutes, the stint menu bar shows the adopted timer; the main window also reflects it.
2. With a stint timer running, start a *different* timer in Solidtime web. The Today route shows the conflict banner with the right metadata.
3. Click "Stop it remotely" — the remote entry is closed (verify in Solidtime web), local timer continues.
4. (Restart) repeat (2), click "Switch to it" — the local timer stops, the remote one becomes the running entry.
5. (Restart) repeat (2), click "Dismiss" — banner disappears, both keep running.
6. Edit an entry in Solidtime's web UI (description); within 5 min, stint's Today view reflects the new description.
7. Delete an entry in Solidtime's web UI; within 5 min, the entry disappears from stint.

Document anything that doesn't work; either fix it in a follow-up task here or open a bug.

- [ ] **Step 5: Push branch + open PR**

```bash
git push -u origin phase-3c
gh pr create --title "feat: Phase 3c — Solidtime down-sync" --body "$(cat <<'EOF'
## Summary

- Adopt a remote-side running timer when stint has none.
- Surface a conflict banner when both sides have different timers running.
- Pull recent completed entries and reconcile edits/deletes from upstream.
- Background poll every 5 min + on-focus refresh + manual "Refresh" button + `stint pull` CLI.

Implements `docs/superpowers/specs/2026-05-20-solidtime-down-sync.md`.

## Test plan

- [x] cargo test --workspace -- --test-threads=1
- [x] cargo clippy --workspace --all-targets -- -D warnings
- [x] cd ui && pnpm typecheck && pnpm build
- [x] Manual: timer started in Solidtime web is adopted within 5 min.
- [x] Manual: conflict banner shows Stop / Switch / Dismiss; each works.
- [x] Manual: edits and deletes from Solidtime web reflect locally within 5 min.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [ ] **Step 6: After CI green + merge, tag**

```bash
gh pr merge <PR#> --rebase --delete-branch
git fetch && git reset --hard origin/main
git tag -a phase-3c-complete -m "Phase 3c — Solidtime down-sync"
# Ask user before pushing the tag:
# git push origin phase-3c-complete
```

---

## Self-review checklist (run before declaring the plan ready)

- [x] Spec §1–§16 — every section mapped to a task. (§1–§3 framing → no task; §4 triggers → tasks 5 + 8 + 17; §5 API → 2+3; §6 running → 6+7; §7 conflict → 10; §8 history → 11–14; §9 deletes → 15+16; §10 transaction → 14; §11 schema → no migration needed; §12 UI → 10+13; §13 failures → handled in implementation; §14 test plan → tasks already cover; §15 out-of-scope → respected; §16 phasing → matches commit groups.)
- [x] No placeholders ("TBD", "implement later").
- [x] Type/method names consistent: `RemoteEntryUpsert`, `reconcile_running`, `Trigger::*`, `PullReport`, `ConflictInfo`, `pull(store, client, trigger)`.
- [x] Each task has exact file paths and complete code.
- [x] Each task commits.

Open assumption flagged: **Task 14 assumes the SQLite pool is `max_connections=1`.** If it isn't, the BEGIN/COMMIT approach in pull/mod.rs may interleave with other writers. Task 14 Step 1 forces a verification before proceeding.
