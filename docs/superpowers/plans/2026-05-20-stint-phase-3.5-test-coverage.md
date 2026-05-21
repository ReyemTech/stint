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

- [ ] **Task 13: Add app test harness and shared setup**

**Files:**
- Add: `crates/stint-app/tests/common/mod.rs`
- Add: a first app integration test file, likely `crates/stint-app/tests/timer_commands.rs`
- Modify only if needed: `crates/stint-app/Cargo.toml`

**Steps:**
- [ ] Mirror the `stint-core/tests/common/mod.rs` tempdir-store setup pattern for app tests.
- [ ] Add only the dependencies needed to test command helpers outside a real Tauri runtime.
- [ ] Prove the harness with one minimal failing/passing test.
- [ ] Commit: `test(app): add command helper test harness`

- [ ] **Task 14: Extract and cover `timer.rs` command bodies**

**Files:**
- Modify: `crates/stint-app/src/commands/timer.rs`
- Modify: `crates/stint-app/tests/timer_commands.rs` or equivalent

**Steps:**
- [ ] Extract `get_running_timer`, `start_timer`, `stop_timer`, `delete_entry`, `update_description`, `set_entry_project`, and `set_entry_billable` bodies into `pub(crate)` helpers.
- [ ] Leave `#[tauri::command]` wrappers thin.
- [ ] Test the helpers directly against temp stores.
- [ ] Commit: `refactor(app): extract timer command helpers`

- [ ] **Task 15: Extract and cover low-risk command modules**

**Files:**
- Modify: `crates/stint-app/src/commands/entries.rs`
- Modify: `crates/stint-app/src/commands/projects.rs`
- Modify: `crates/stint-app/src/commands/sync.rs`
- Add/modify matching test files under `crates/stint-app/tests/`

**Steps:**
- [ ] Extract helper bodies in the same pattern as Task 14.
- [ ] Focus on commands that are store/client driven and do not require browser launch or real Tauri UI state.
- [ ] Add direct helper tests and re-measure coverage.
- [ ] Commit: `refactor(app): extract entries and sync helpers`

- [ ] **Task 16: Extract and cover selected config/pull/calendar helpers**

**Files:**
- Modify: `crates/stint-app/src/commands/config.rs`
- Modify: `crates/stint-app/src/commands/pull.rs`
- Modify: `crates/stint-app/src/commands/calendar.rs`
- Add/modify matching test files under `crates/stint-app/tests/`

**Steps:**
- [ ] Cover only the non-browser portions:
  - config read/write/test paths
  - pull/report/resolve paths
  - calendar list/refresh/log/ignore paths that do not open a browser
- [ ] Leave OAuth-start/browser-launch wrappers thin and effectively exempt.
- [ ] Re-run crate coverage and stop once extracted logic reaches >=80%.
- [ ] Commit: `test(app): reach command helper coverage target`

**Review gates for Tasks 13-16:**
- [ ] Spec-compliance review confirms wrapper-thinning is happening instead of moving business logic deeper into Tauri-only code.
- [ ] Code-quality review confirms helper extraction improves design rather than creating parallel abstractions with no reuse value.

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
12. `test(app): add command helper test harness`
13. `refactor(app): extract timer command helpers`
14. `refactor(app): extract entries and sync helpers`
15. `test(app): reach command helper coverage target`
16. `test(ui): add vitest harness`
17. `test(ui): cover stores and lib helpers`
18. `test(ui): cover route helper logic`
19. `test: finalize coverage uplift`

Combine or split only if coverage data shows a task boundary is too coarse or too fine.
