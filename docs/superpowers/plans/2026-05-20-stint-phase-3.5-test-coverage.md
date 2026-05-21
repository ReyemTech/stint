# Phase 3.5 — Test coverage uplift Implementation Plan

> **For agentic workers:** REQUIRED WORKFLOW: use fresh subagents for implementation tasks during execution. Each task gets (1) implementation, (2) spec-compliance review, and (3) code-quality review before merge. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Raise automated coverage across `stint-core`, `stint-cli`, `stint-app`, and `ui` to the thresholds defined in `docs/superpowers/specs/2026-05-20-test-coverage-uplift.md`, while leaving the codebase materially easier to test for Phase 3d.

**Architecture:** Keep `stint-core` as the business-logic center, extend the existing integration-style test pattern around tempdir SQLite + wiremock, broaden CLI `assert_cmd` coverage, extract `stint-app` command bodies into directly testable helpers, and add a Vitest/jsdom harness for UI pure logic. Infinite loop drivers stay thin and mostly uncovered; one-tick helpers carry the meaningful coverage.

**Tech stack:** Rust 1.95 · `cargo-llvm-cov` with rustup LLVM tools · `assert_cmd` · `wiremock` · Tauri 2 command helpers tested outside the runtime shell · Vitest + jsdom + `@solidjs/testing-library` · pnpm in `ui/`.

**Spec:** `docs/superpowers/specs/2026-05-20-test-coverage-uplift.md`

---

## Workflow guardrails

- One fresh worker per task during execution; do not reuse a worker across unrelated file ownership.
- Worker ownership must be explicit. Each worker owns only the files named in its task.
- Every task follows TDD where feasible: write failing tests, confirm the fail, implement minimum code, confirm the pass.
- After each task, run the narrowest useful coverage command and confirm the targeted file or crate moved up.
- After each task, run two reviews before finalizing:
  - spec-compliance review
  - code-quality review
- Stop chasing coverage on a surface once the threshold is met cleanly and only exempt glue remains.

---

## Phase placement and branch

This phase lands as **3.5**, not `2.6`:

- it is a tooling/test phase in the decimal convention
- it sits chronologically between shipped `3c` and planned `3d`
- its output is explicitly in service of the upcoming Phase 3d UX work

Branch from `main`:

```bash
git checkout main
git pull --ff-only
git checkout -b phase-3.5
```

---

## Coverage commands

### Rust workspace summary

```bash
LLVM_COV="$(ls -d ~/.rustup/toolchains/*/lib/rustlib/aarch64-apple-darwin/bin)/llvm-cov"
LLVM_PROFDATA="$(ls -d ~/.rustup/toolchains/*/lib/rustlib/aarch64-apple-darwin/bin)/llvm-profdata"

LLVM_COV="$LLVM_COV" LLVM_PROFDATA="$LLVM_PROFDATA" \
  cargo llvm-cov --workspace --summary-only \
  --ignore-filename-regex 'tests/|crates/stint-cli/|crates/stint-app/' \
  -- --test-threads=1
```

### Rust per-file detail

```bash
LLVM_COV="$LLVM_COV" LLVM_PROFDATA="$LLVM_PROFDATA" \
  cargo llvm-cov -p stint-core -- --test-threads=1
```

Use crate-scoped runs for `stint-cli` and `stint-app` once those crates have their own tests.

### UI tests and coverage

```bash
cd ui
pnpm test --run
pnpm test:coverage --run
```

If the final scripts differ slightly after setup, normalize them before phase close so the README/plan commands remain truthful.

---

## Pre-flight

- [ ] **Task 1: Baseline the phase and create the execution branch**

**Files:**
- Modify: `README.md` and `CLAUDE.md` only if the roadmap/status drifted again before execution
- No code changes otherwise

**Steps:**
- [ ] Confirm the worktree state and branch from `main` to `phase-3.5`.
- [ ] Capture baseline coverage snapshots for:
  - `stint-core` workspace summary
  - crate-level `stint-cli`
  - crate-level `stint-app`
  - current `ui/` test situation (expected: no test script)
- [ ] Record the numbers in the PR description or task log during execution; no committed artifact required unless helpful.
- [ ] Run the current baseline suite:
  - `cargo test --workspace -- --test-threads=1`
  - `cd ui && pnpm typecheck`
- [ ] Commit only if roadmap docs changed; otherwise move on without a commit.

**Review gates:**
- [ ] Spec-compliance review confirms the baseline commands match the spec.
- [ ] Code-quality review confirms no accidental code churn happened during setup.

---

## `stint-core` uplift

- [ ] **Task 2: Cover `paths.rs` override and error branches**

**Files:**
- Modify: `crates/stint-core/tests/store_connect.rs` or add `crates/stint-core/tests/paths.rs`
- Modify only if needed: `crates/stint-core/src/paths.rs`

**Steps:**
- [ ] Add failing tests for env override resolution and the relevant error path(s).
- [ ] Implement only the minimum production change needed, if any.
- [ ] Run the narrow test file and then crate coverage to confirm `paths.rs` moved materially above its current ~57%.
- [ ] Commit: `test(core): cover path resolution branches`

- [ ] **Task 3: Cover `solidtime/mod.rs` auth and failure branches**

**Files:**
- Modify: `crates/stint-core/tests/solidtime.rs`
- Modify only if needed: `crates/stint-core/src/solidtime/mod.rs`

**Steps:**
- [ ] Add wiremock-backed failures for 401, representative non-401 error responses, and any currently missed branch around request handling.
- [ ] Reuse the existing client fixtures rather than inventing a second pattern.
- [ ] Re-run crate coverage and confirm the file moves above its current ~79%.
- [ ] Commit: `test(core): cover solidtime error branches`

- [ ] **Task 4: Cover `calendar/google/mod.rs` provider construction branches**

**Files:**
- Modify: `crates/stint-core/tests/calendar_google_provider.rs`
- Modify only if needed: `crates/stint-core/src/calendar/google/mod.rs`

**Steps:**
- [ ] Add tests for `build_provider_from_blob` success, malformed blob, missing token fields, and provider config-disabled paths.
- [ ] Keep this task focused on provider construction, not refresh behavior.
- [ ] Confirm coverage movement for `calendar/google/mod.rs`.
- [ ] Commit: `test(core): cover google provider setup branches`

- [ ] **Task 5: Cover `timer.rs` mutation helpers**

**Files:**
- Modify: `crates/stint-core/tests/timer.rs`
- Modify only if needed: `crates/stint-core/src/timer.rs`

**Steps:**
- [ ] Add failing tests for:
  - `delete`
  - `update_description`
  - `set_project`
  - `set_billable`
  - `maybe_enqueue_update`
- [ ] Assert both store-state effects and queue side effects.
- [ ] Confirm `timer.rs` moves well above its current ~64%.
- [ ] Commit: `test(core): cover timer mutation paths`

- [ ] **Task 6: Cover `recovery.rs` decision branches**

**Files:**
- Modify: `crates/stint-core/tests/recovery.rs`
- Modify only if needed: `crates/stint-core/src/recovery.rs`

**Steps:**
- [ ] Add tests for discard / keep / stop-at-heartbeat branches and the user-prompt outcomes that steer them.
- [ ] Keep the prompt seam mocked; do not introduce UI concerns here.
- [ ] Confirm `recovery.rs` moves above its current ~66%.
- [ ] Commit: `test(core): cover recovery branch matrix`

- [ ] **Task 7: Cover `calendar/sync.rs` range variants**

**Files:**
- Modify: `crates/stint-core/tests/calendar_sync.rs`
- Modify only if needed: `crates/stint-core/src/calendar/sync.rs`

**Steps:**
- [ ] Add tests for each `Ranges` variant flowing through `refresh_account`.
- [ ] Verify the correct time windows, inclusion filters, and persistence behavior.
- [ ] Confirm `calendar/sync.rs` improves beyond its current ~76%.
- [ ] Commit: `test(core): cover calendar sync ranges`

- [ ] **Task 8: Extract and cover one-tick sync logic**

**Files:**
- Modify: `crates/stint-core/src/sync/mod.rs`
- Add or modify: `crates/stint-core/tests/sync_push.rs` and/or a new targeted sync test

**Steps:**
- [ ] Extract one-tick logic from `run_loop` into a helper analogous to the existing `pull_worker` pattern.
- [ ] Test the helper rather than the infinite driver.
- [ ] Confirm `sync/mod.rs` moves materially from its current ~37% without trying to fake an infinite-loop test.
- [ ] Commit: `refactor(core): extract sync tick for coverage`

- [ ] **Task 9: Re-run `stint-core` coverage and close only the cheapest remaining gap**

**Files:**
- Whatever single `stint-core` file still blocks >=90%, kept intentionally narrow

**Steps:**
- [ ] Run crate coverage and rank the remaining misses.
- [ ] Pick the smallest high-value gap that gets `stint-core` to >=90%.
- [ ] Add tests, re-measure, and stop once threshold is reached.
- [ ] Commit: `test(core): reach coverage target`

**Review gates for Tasks 2-9:**
- [ ] Spec-compliance review after each task confirms the task stayed within the planned file scope and honored the exemption rules.
- [ ] Code-quality review after each task confirms no production complexity was added solely to game coverage.

---

## `stint-cli` uplift

- [ ] **Task 10: Expand CLI harness for timer/history commands**

**Files:**
- Modify: `crates/stint-cli/tests/cli_e2e.rs`
- Modify only if needed: `crates/stint-cli/src/cmd/start.rs`, `stop.rs`, `today.rs`, `list.rs`

**Steps:**
- [ ] Add `assert_cmd` coverage for `start`, `stop`, `today`, and `list`.
- [ ] Use `STINT_DB` tempdirs and assert on meaningful output/state, not every whitespace detail.
- [ ] Confirm crate coverage improves materially from the current near-zero baseline.
- [ ] Commit: `test(cli): cover timer and history commands`

- [ ] **Task 11: Expand CLI harness for mutation/config commands**

**Files:**
- Modify: `crates/stint-cli/tests/cli_e2e.rs`
- Modify existing tests if needed: `crates/stint-cli/tests/cli_login.rs`
- Modify only if needed: `crates/stint-cli/src/cmd/edit.rs`, `delete.rs`, `config.rs`, `projects.rs`

**Steps:**
- [ ] Add coverage for `edit`, `delete`, `config`, and `projects`.
- [ ] Reuse any existing login/config test helpers instead of duplicating env setup.
- [ ] Keep browser-interactive login flow out of scope except for existing smoke behavior.
- [ ] Commit: `test(cli): cover config and mutation commands`

- [ ] **Task 12: Expand CLI harness for sync/pull/calendar**

**Files:**
- Modify: `crates/stint-cli/tests/cli_e2e.rs`
- Modify existing tests if needed: `crates/stint-cli/tests/cli_calendar.rs`
- Modify only if needed: `crates/stint-cli/src/cmd/sync.rs`, `pull.rs`, `calendar.rs`

**Steps:**
- [ ] Add wiremock-backed tests for `sync`, `pull`, and the non-browser `calendar` subcommands.
- [ ] Keep the interactive OAuth calendar add flow under smoke-level expectations only.
- [ ] Re-run crate coverage and stop once `stint-cli` reaches >=80% with `main.rs` as the obvious remaining miss.
- [ ] Commit: `test(cli): reach coverage target`

**Review gates for Tasks 10-12:**
- [ ] Spec-compliance review confirms every subcommand named in the spec has coverage or an explicit exemption.
- [ ] Code-quality review confirms test helpers are being consolidated instead of copied.

---

## `stint-app` uplift

**Approach revision (2026-05-20):** the original plan called for extracting every `#[tauri::command]` body into a `pub(crate)` helper so tests could call the helper against a tempdir Store. After realising Tauri 2 ships `tauri::test::mock_builder()` — which gives a real `App<MockRuntime>` with working `AppHandle` and `State` — the helper-extraction refactor isn't needed. The new approach: dev-feature `tauri/test`, build a mock app per test, call the `#[tauri::command]` functions directly. Production code stays untouched; coverage shape stays the same.

- [ ] **Task 13: Add Tauri mock-app test harness**

**Files:**
- Modify: `crates/stint-app/Cargo.toml` (add `tauri/test` feature to dev-dependencies)
- Add: `crates/stint-app/tests/common/mod.rs`
- Add: `crates/stint-app/tests/timer_commands.rs` (one proof-of-life test)

**Steps:**
- [ ] In `crates/stint-app/Cargo.toml`, add the test feature to dev-dependencies:
  ```toml
  [dev-dependencies]
  tauri = { version = "2.1", features = ["test"] }
  tempfile.workspace = true
  tokio.workspace = true
  ```
  Cargo unifies features, so the main `tauri` dependency keeps its production feature set while tests additionally see `test`.
- [ ] Create `crates/stint-app/tests/common/mod.rs` exposing:
  - `fn mock_app() -> tauri::App<tauri::test::MockRuntime>` — uses `tauri::test::mock_builder()` + `mock_context(noop_assets())`.
  - `async fn fresh_store() -> (TempDir, Arc<Store>)` — tempdir-backed store, mirroring `stint-core/tests/common/mod.rs::setup`.
  - A composer like `async fn make_app_with_state() -> AppContext` returning the mock app, store, and a held `TempDir` (so the tempdir survives the test scope). Manage `RwLock<AppState>` on the app handle inside the helper.
- [ ] Write one proof-of-life test in `tests/timer_commands.rs`: call `get_running_timer(handle, state)` against a fresh store, assert `Ok(None)`.
- [ ] Verify the mock app builds and the test passes: `cargo test -p stint-app -- --test-threads=1`.
- [ ] Commit: `test(app): add tauri mock-app test harness`

- [ ] **Task 14: Cover `timer.rs` commands**

**Files:**
- Modify: `crates/stint-app/tests/timer_commands.rs`
- No production-code changes expected.

**Steps:**
- [ ] Using the mock harness, add direct tests for each `#[tauri::command]` in `commands/timer.rs`:
  - `start_timer` happy path → returns local_uuid, entry persisted, queue op enqueued.
  - `start_timer` while running → returns Err with the "already running" invariant.
  - `stop_timer` happy path → entry's `end_at` is set, running_timer cleared.
  - `delete_entry` on pending_create → hard delete.
  - `update_description`, `set_entry_project`, `set_entry_billable` round trips.
- [ ] Assert event emission where helpful: `MockRuntime` captures `app.emit()` calls; verify `entries:changed` fires on mutations.
- [ ] Re-run crate coverage and confirm `commands/timer.rs` is materially covered.
- [ ] Commit: `test(app): cover timer commands`

- [ ] **Task 15: Cover `entries.rs`, `projects.rs`, `sync.rs` commands**

**Files:**
- Add: `crates/stint-app/tests/entries_commands.rs`
- Add: `crates/stint-app/tests/projects_commands.rs`
- Add: `crates/stint-app/tests/sync_commands.rs`
- No production-code changes expected.

**Steps:**
- [ ] `entries_commands.rs`: cover `list_today`, `list_between` against seeded entries.
- [ ] `projects_commands.rs`: cover `list_projects` (empty + seeded), `list_organizations` and `refresh_projects` against a wiremock Solidtime.
- [ ] `sync_commands.rs`: cover `sync_now` happy path (wiremock-backed) + missing-config error path.
- [ ] Re-run crate coverage; aggregate moves up materially.
- [ ] Commit: `test(app): cover entries, projects, sync commands`

- [ ] **Task 16: Cover `config.rs`, `pull.rs`, and non-browser `calendar.rs`**

**Files:**
- Add: `crates/stint-app/tests/config_commands.rs`
- Add: `crates/stint-app/tests/pull_commands.rs`
- Add: `crates/stint-app/tests/calendar_commands.rs`
- No production-code changes expected.

**Steps:**
- [ ] `config_commands.rs`: cover `config_show`, `config_set`, `config_test`, `solidtime_url`. Use `STINT_SECRET_PREFIX` to route the `Secrets::default()` writes inside the binary code to a synthetic prefix — the test process doesn't need to clean up; the synthetic entries are swept by `scripts/clean-test-keychain.sh`.
- [ ] `pull_commands.rs`: cover `pull_now` (wiremock-backed: empty remote, single insert, conflict detected) and `conflict_resolve` for `dismiss` / `stop_remote` / `switch` actions.
- [ ] `calendar_commands.rs`: cover `calendar_list_accounts`, `calendar_list_calendars`, `calendar_set_calendar_included`, `calendar_remove_account`, `calendar_list_events_in_range`, `calendar_log_event`, `calendar_ignore_event`. Skip `calendar_add_google` (interactive OAuth) and `calendar_oauth_status` for accounts whose blob doesn't exist (NoEntry branch is enough).
- [ ] Re-run crate coverage and stop once `stint-app` reaches >=80% on the non-browser surface. `calendar_add_google` and `oauth_solidtime_start` remain exempt per the spec.
- [ ] Commit: `test(app): reach app coverage target`

**Review gates for Tasks 13-16:**
- [ ] Spec-compliance review confirms no production code changes leaked in (these tasks are test-only after the harness lands).
- [ ] Code-quality review confirms shared setup is reused across test files rather than copy-pasted.

**Notes on the mock runtime:**
- `MockRuntime` doesn't actually run the sync_worker / pull_worker background tasks. `sync_worker::nudge(...)` etc. become no-ops in tests, which is fine — assertions focus on database state and event emission, not worker side-effects.
- `app.emit("entries:changed", ())` calls succeed silently in tests. To assert emission, use the listener pattern: `app.listen("entries:changed", |event| { /* record */ })` before invoking the command.
- The `AppHandle::state::<T>()` lookup works exactly like in production — register state with `handle.manage(RwLock::new(AppState { ... }))` in the harness.

---

## UI uplift

- [ ] **Task 17: Add UI test infrastructure**

**Files:**
- Modify: `ui/package.json`
- Add: `ui/vitest.config.ts`
- Add: `ui/src/test/setup.ts` or equivalent
- Modify only if needed: `ui/tsconfig.json` / `ui/tsconfig.*`

**Steps:**
- [ ] Add `vitest`, `jsdom`, and `@solidjs/testing-library`.
- [ ] Add `test` and `test:coverage` scripts.
- [ ] Mirror the Vite `~/` alias into Vitest config.
- [ ] Confirm `pnpm test --run` executes successfully from `ui/`.
- [ ] Commit: `test(ui): add vitest harness`

- [ ] **Task 18: Cover store and lib pure logic**

**Files:**
- Add: `ui/src/stores/timer.test.ts`
- Add: `ui/src/lib/openSolidtime.test.ts`
- Add: `ui/src/lib/useHotkey.test.ts` or a narrower extracted helper test if the hook itself is awkward
- Modify production files only if a tiny helper extraction improves testability

**Steps:**
- [ ] Prioritize deterministic pure logic and signal-store behavior.
- [ ] Avoid DOM-heavy component work unless a helper cannot be tested otherwise.
- [ ] Re-run UI coverage and confirm meaningful non-zero movement.
- [ ] Commit: `test(ui): cover stores and lib helpers`

- [ ] **Task 19: Extract and cover route helpers**

**Files:**
- Modify: `ui/src/routes/Today.tsx`
- Modify: `ui/src/routes/Popover.tsx`
- Modify: `ui/src/routes/Settings.tsx`
- Add: route helper modules/tests as needed, for example `ui/src/routes/today-helpers.ts` and `*.test.ts`

**Steps:**
- [ ] Extract pure formatting/filtering/state-derivation helpers from route files where worthwhile.
- [ ] Test those helpers directly.
- [ ] Stop once UI reaches practical logic coverage and additional gains would mostly be brittle component tests.
- [ ] Commit: `test(ui): cover route helper logic`

**Review gates for Tasks 17-19:**
- [ ] Spec-compliance review confirms the UI work stayed logic-first and did not drift into snapshot churn.
- [ ] Code-quality review confirms helper extraction preserved the existing UI patterns and aliasing.

---

## Final sweep and release prep

- [ ] **Task 20: Verify thresholds, run full suites, and prep the PR**

**Files:**
- Modify only if needed: tiny follow-up tests or coverage-script normalization
- Optionally update: `README.md` / `CLAUDE.md` if developer commands or testing guidance changed materially

**Steps:**
- [ ] Run final Rust coverage checks and confirm:
  - `stint-core` >= 90%
  - `stint-cli` >= 80%
  - `stint-app` >= 80% on testable logic
- [ ] Run final UI coverage and confirm meaningful non-zero logic coverage with the target roughly met.
- [ ] Run:
  - `cargo test --workspace -- --test-threads=1`
  - `cd ui && pnpm test --run`
- [ ] Run any final lint/typecheck needed to keep CI green:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cd ui && pnpm typecheck`
- [ ] Open/update the PR and summarize the final coverage deltas.
- [ ] Wait for CI green before merge.
- [ ] After merge to `main`, tag `phase-3.5-complete`.
- [ ] Ask before pushing the tag.

**Review gates:**
- [ ] Spec-compliance review confirms all explicit targets or exemptions were honored.
- [ ] Code-quality review confirms no last-minute coverage hacks slipped in.

---

## Expected commit series

1. `test(core): cover path resolution branches`
2. `test(core): cover solidtime error branches`
3. `test(core): cover google provider setup branches`
4. `test(core): cover timer mutation paths`
5. `test(core): cover recovery branch matrix`
6. `test(core): cover calendar sync ranges`
7. `refactor(core): extract sync tick for coverage`
8. `test(core): reach coverage target`
9. `test(cli): cover timer and history commands`
10. `test(cli): cover config and mutation commands`
11. `test(cli): reach coverage target`
12. `test(app): add tauri mock-app test harness`
13. `test(app): cover timer commands`
14. `test(app): cover entries, projects, sync commands`
15. `test(app): reach app coverage target`
16. `test(ui): add vitest harness`
17. `test(ui): cover stores and lib helpers`
18. `test(ui): cover route helper logic`
19. `test: finalize coverage uplift`

Combine or split only if coverage data shows a task boundary is too coarse or too fine.
