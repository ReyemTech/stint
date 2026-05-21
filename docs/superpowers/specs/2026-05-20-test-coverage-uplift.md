# stint — Test coverage uplift (spec)

Raise automated test coverage across the Rust workspace and establish a real UI test harness so post-3c work can ship with tighter regression protection.

- **Status:** Draft 2026-05-20
- **Predecessors:** Phase 2.5 (CI baseline), Phase 3c (Solidtime down-sync)
- **Target placement:** Standalone tooling phase between 3c and 3d, **Phase 3.5**

## 1. Problem

Coverage is uneven across the repo:

- `stint-core` is healthy but still misses important branches in sync, timer, auth, calendar-provider setup, and recovery code.
- `stint-cli` has only four smoke tests, so command routing and output shape regressions are cheap to introduce.
- `stint-app` has effectively no automated coverage because testable logic still lives inside `#[tauri::command]` bodies tied to `State<'_, RwLock<AppState>>` and `AppHandle`.
- `ui` has no test runner, so all regressions are caught manually.

That was tolerable while shipping foundations. It is not a good base for Phase 3d UX work, which will touch all four surfaces and add more stateful behavior.

## 2. Goals

### Primary

1. Raise **`stint-core` line coverage to at least 90%**.
2. Raise **`stint-cli` line coverage to at least 80%**, excluding the binary entrypoint dispatch in `main.rs`.
3. Raise **`stint-app` line coverage to at least 80% on testable logic**, by extracting command bodies into testable helpers and leaving `#[tauri::command]` functions as thin shells.
4. Establish a **UI unit-test harness** with Vitest + jsdom + `@solidjs/testing-library`, then cover pure logic first (`stores/`, `lib/`, route helpers) with a practical target of **around 60%** on files worth testing.

### Secondary

- Make coverage collection repeatable for local development and CI follow-up work.
- Keep new tests cheap to extend during Phase 3d and later decimal/tooling phases.
- Prefer integration-style tests at the store / command-helper boundary over brittle shallow unit tests.

## 3. Non-goals

- Chasing 100% coverage.
- Adding tests around `main.rs` entrypoints whose value is mostly argv parsing + dispatch.
- Unit-testing infinite worker loops themselves (`run_loop`, `*_worker::spawn`); only the single-tick logic they delegate to needs coverage.
- Browser-driven OAuth flows (`login_interactive`, Google/Tauri OAuth open-browser commands) beyond the existing smoke path.
- Full GUI end-to-end automation for Tauri windows.
- Hands-on UAT as part of this phase. The output is tests + coverage, not a user-facing feature.

## 4. Coverage targets and stop conditions

| Surface | Baseline | Target | Stop condition |
|---|---:|---:|---|
| `stint-core` | 87.4% lines | **>= 90%** | Stop once workspace coverage confirms `stint-core` at or above target. |
| `stint-cli` | effectively 0% | **>= 80%** | Stop once subcommand coverage is broad and entrypoint-only gaps remain. |
| `stint-app` | 0% | **>= 80%** on extracted logic | Stop once helpers / services reach target and only thin Tauri shells remain largely uncovered. |
| `ui` | 0% | **practical logic coverage**, target ~60% | Stop once pure logic is covered and further gains would require brittle component-heavy tests. |

The phase is threshold-driven. Once the target for a surface is reached cleanly, the plan should move on instead of grinding out low-value tests.

## 5. Coverage measurement

Workspace coverage is measured with `cargo-llvm-cov` using the rustup-provided LLVM tools:

```bash
LLVM_COV="$(ls -d ~/.rustup/toolchains/*/lib/rustlib/aarch64-apple-darwin/bin)/llvm-cov"
LLVM_PROFDATA="$(ls -d ~/.rustup/toolchains/*/lib/rustlib/aarch64-apple-darwin/bin)/llvm-profdata"

LLVM_COV="$LLVM_COV" LLVM_PROFDATA="$LLVM_PROFDATA" \
  cargo llvm-cov --workspace --summary-only \
  --ignore-filename-regex 'tests/|crates/stint-cli/|crates/stint-app/' \
  -- --test-threads=1
```

Per-file inspection drops `--summary-only`. CLI and app progress should also be checked with crate-scoped coverage runs that include their own source files once tests exist there.

UI coverage uses Vitest's built-in coverage reporter and should be scoped to `ui/src/**`, excluding generated types and obvious glue files where appropriate.

## 6. Per-surface scope

### 6.1 `stint-core`

Primary focus is the current high-value gap list:

- `sync/mod.rs`: cover extracted single-tick logic rather than the infinite `run_loop`.
- `calendar/google/mod.rs`: cover `build_provider_from_blob` happy/error branches.
- `paths.rs`: env override and filesystem error branches.
- `timer.rs`: `delete`, `update_description`, `set_project`, `set_billable`, `maybe_enqueue_update`.
- `recovery.rs`: discard / keep / stop-at-heartbeat decisions.
- `calendar/sync.rs`: `refresh_account` with the different `Ranges` variants.
- `solidtime/mod.rs`: auth and error branches.

Testing style stays the same as existing core work:

- real SQLite tempdir via `tests/common/mod.rs`
- wiremock for HTTP shape and server responses
- narrow unit tests only where the logic is pure and store setup would be noise

### 6.2 `stint-cli`

Add `assert_cmd` integration coverage for the command surface:

- `start`
- `stop`
- `today`
- `list`
- `edit`
- `delete`
- `config`
- `projects`
- `sync`
- `pull`
- `calendar`

Tests should use:

- tempdir-backed DB via `STINT_DB`
- existing keychain-test guard conventions where needed
- wiremock Solidtime responses for networked flows

The goal is broad command-path confidence, not exhaustive golden-output testing of every line of terminal formatting.

### 6.3 `stint-app`

The key design change for this phase is **command-body extraction**:

```rust
pub(crate) async fn do_x(store: &Store, ...) -> Result<...>
```

Pattern:

1. Move command logic out of `#[tauri::command]` functions.
2. Keep wrappers responsible only for unwrapping `State<'_, RwLock<AppState>>`, obtaining the `Store` / dependencies, and forwarding arguments.
3. Test the extracted helpers directly against a tempdir-backed store, wiremock Solidtime, and mock provider seams where needed.

This phase explicitly allows thin Tauri wrappers themselves to remain lightly covered.

Testing support should mirror the core test setup pattern in a new `crates/stint-app/tests/common/mod.rs` helper so app tests can stand up a reusable in-memory-ish temp store arrangement without depending on a real Tauri runtime.

### 6.4 `ui`

Add test infrastructure:

- `vitest`
- `jsdom` (or `happy-dom`; default to `jsdom` unless the setup fights Solid too hard)
- `@solidjs/testing-library`

Add `vitest.config.ts` mirroring the Vite alias:

- `~/` -> `ui/src/`

Coverage priority order:

1. `ui/src/stores/*`
2. `ui/src/lib/*`
3. pure helper logic in `ui/src/routes/*`
4. only then selective component interaction tests where the payoff is obvious

Snapshot-heavy testing is out of scope unless a component has meaningful behavior that cannot be covered any other way.

## 7. Known exemptions and pragmatic caveats

- `sync/mod.rs::run_loop` and analogous worker loops stay effectively uncovered; extract and test one-tick helpers instead.
- `stint-cli/src/main.rs` and `stint-app/src/main.rs` are exempt from coverage targets.
- Interactive OAuth / browser-launch flows are not worth forcing into unit tests; cover token persistence, exchange, refresh, and related non-interactive logic instead.
- Some Tauri commands that only spawn a browser or emit shell-side effects may remain wrapper-only and be excluded from meaningful coverage expectations.
- `stint-app` and `ui` targets are bounded by cleanliness. If the only way to hit the last few percent is invasive architecture churn or brittle DOM tests, the phase should stop once the agreed threshold is reached.

## 8. Delivery workflow

This phase follows the same shape used for 3c:

1. Spec.
2. Detailed task plan with bite-sized, mostly one-file-at-a-time TDD tasks.
3. Subagent-driven execution, one fresh worker per task where practical.
4. Two review passes per task:
   - spec-compliance review
   - code-quality review
5. End-of-phase verification:
   - `cargo test --workspace -- --test-threads=1`
   - `pnpm test --run`
   - coverage verification against the thresholds above

The repository in this session does not expose the named `superpowers:*` skills directly, so the implementation plan should mirror that workflow explicitly in the plan document and agent orchestration rather than depending on a missing skill hook.

## 9. Expected outputs

- New spec: this document.
- New plan doc for the testing-uplift phase.
- New or expanded Rust tests across all three crates.
- Extracted `stint-app` helper functions that make command logic testable.
- UI test harness and initial logic-focused suites.
- Updated roadmap/status entries in `README.md` and `CLAUDE.md`.

## 10. Success criteria

The phase is successful when all of the following are true:

1. `stint-core` coverage is at or above 90% lines.
2. `stint-cli` coverage is at or above 80% lines, with remaining misses concentrated in intentionally exempt entrypoint code.
3. `stint-app` testable logic is at or above 80% lines via extracted helper coverage.
4. UI test infrastructure exists, runs locally, and covers the highest-value pure logic with meaningful non-zero coverage.
5. Full workspace tests and UI tests pass at phase end.
6. The codebase is left easier to test during Phase 3d than it was before this phase began.
