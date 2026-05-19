# stint Phase 2.5: CI Baseline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a GitHub Actions CI workflow on a macOS runner that runs `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `pnpm typecheck`, and `pnpm build` on every PR and push to `main`. Required as a status check on PRs merging to `main`. Warm-cache runs target under 5 min.

**Architecture:** Single workflow file (`.github/workflows/ci.yml`) with a single job on `macos-14`. Cargo caching via `Swatinem/rust-cache@v2`; pnpm caching via `actions/setup-node@v4`'s built-in pnpm cache keyed on `**/pnpm-lock.yaml` (covers both lockfiles). The one Keychain-touching test in `crates/stint-core/tests/config.rs` is gated behind a `STINT_SKIP_KEYCHAIN_TESTS` env var which CI sets and local dev does not.

**Tech Stack:** GitHub Actions · `actions/checkout@v4` · `dtolnay/rust-toolchain@stable` (with explicit `toolchain: "1.81"`) · `Swatinem/rust-cache@v2` · `pnpm/action-setup@v4` · `actions/setup-node@v4`.

---

## Why env-gated skip (not a mock backend)

The spec at §9 lists the CI scope; it does not prescribe how to handle the single Keychain-touching test (`set_get_delete_round_trip` in `crates/stint-core/tests/config.rs:14`). Two options were considered:

1. **Env-gated skip** (chosen). Three-line guard at the top of the one test; `STINT_SKIP_KEYCHAIN_TESTS=1` set in the workflow's job env. Zero changes to `stint-core` source; local dev loop unchanged.
2. **Mock `SecretsBackend` trait.** Introduce an abstraction over `keyring::Entry`, swap the implementation under `cfg(test)` or via DI. Buys testing of `Secrets::get/set/delete` round-tripping behind the trait but the round-trip logic is two lines of `match` — there's nothing to test that the `keyring` crate's own tests don't already cover.

We picked (1) because the abstraction in (2) earns no real test coverage and adds an indirection in a tiny module. If a future phase adds a Linux/Windows secrets backend, that's the moment to introduce the trait. YAGNI for now.

The cost of (1) is that the macOS Keychain integration is exercised only on developer machines, not in CI. That's acceptable: Keychain semantics on GitHub-hosted runners (locked-by-default login keychain) don't match end-user environments anyway, so a CI green there would be a false signal.

---

## File Structure

```
stint/
├── .github/
│   └── workflows/
│       └── ci.yml                            # NEW — the entire workflow
├── crates/
│   └── stint-core/
│       └── tests/
│           └── config.rs                     # MODIFIED — add env guard to one test
├── README.md                                 # MODIFIED — flip Phase 2.5 row to shipped
└── CLAUDE.md                                 # MODIFIED — flip Phase 2.5 row + note PR-required workflow
```

After Phase 2.5 lands:

- Every PR targeting `main` triggers a CI run that must pass before merge.
- Every push to `main` triggers a CI run (so the `main` branch always has a current green/red signal).
- Branch protection on `main` requires the `build` check and a PR (no more direct pushes).
- The CLAUDE.md "fast-forward main to the branch" step is replaced by "Rebase and merge" via the GitHub PR UI, which produces the same linear-history result.

---

## Cross-task setup

- **Working directory:** `/Users/mariomeyer/code/ReyemTech/apps/tet`
- **Branch:** `phase-2.5`, branched from `main`.
- **Commits:** Conventional Commits — primarily `chore(ci):` for additions and `fix(ci):` for workflow iterations. `test(core):` for the env-gate change, `docs:` for README/CLAUDE.md.
- **End-state check after each task:** `cargo check --workspace` clean; `pnpm -C ui typecheck` clean if `ui/` was touched (none of these tasks touch `ui/`, so it's a no-op).
- **Tool prereqs (local):** `gh` CLI authenticated (`gh auth status`); Rust toolchain installed (the repo's `rust-toolchain.toml` auto-installs 1.81 on first `cargo` invocation if missing).
- **Push policy:** Until Task 5 lands branch protection, do not push directly to `main`. All commits go on `phase-2.5`.

---

## Tasks

### Task 1: Branch + env-gate the Keychain test

**Files:**
- Modify: `crates/stint-core/tests/config.rs:14-23` (the `set_get_delete_round_trip` test)

- [ ] **Step 1: Confirm clean working tree on `main`**

```bash
git status
git log --oneline -1
```

Expected: `working tree clean`, HEAD at `87d1c96 docs: add CLAUDE.md / AGENTS.md for AI coding agents` (or newer if `main` has advanced).

- [ ] **Step 2: Branch**

```bash
git checkout -b phase-2.5
```

Expected: `Switched to a new branch 'phase-2.5'`.

- [ ] **Step 3: Edit `crates/stint-core/tests/config.rs` to add the env guard**

Replace the body of `set_get_delete_round_trip` (currently lines 14–23) so it reads:

```rust
#[test]
fn set_get_delete_round_trip() {
    // CI does not have a usable macOS Keychain (the login keychain on
    // GitHub-hosted runners is locked by default and prompts differ from
    // end-user behaviour). Local developers run this test; CI sets
    // STINT_SKIP_KEYCHAIN_TESTS=1 so the suite still passes without it.
    if std::env::var("STINT_SKIP_KEYCHAIN_TESTS").is_ok() {
        eprintln!("skipping: STINT_SKIP_KEYCHAIN_TESTS is set");
        return;
    }

    let (secrets, _suffix) = unique_secrets();

    assert!(secrets.get("k").unwrap().is_none());
    secrets.set("k", "hunter2").unwrap();
    assert_eq!(secrets.get("k").unwrap().as_deref(), Some("hunter2"));
    secrets.delete("k").unwrap();
    assert!(secrets.get("k").unwrap().is_none());
}
```

Leave every other test in this file untouched.

- [ ] **Step 4: Verify the test still runs (and passes) without the env var**

```bash
cargo test -p stint-core --test config set_get_delete_round_trip -- --nocapture
```

Expected: `test set_get_delete_round_trip ... ok` (NO "skipping:" line in output).

- [ ] **Step 5: Verify the env var skips it cleanly**

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test -p stint-core --test config set_get_delete_round_trip -- --nocapture
```

Expected: output contains `skipping: STINT_SKIP_KEYCHAIN_TESTS is set` and `test set_get_delete_round_trip ... ok`.

- [ ] **Step 6: Run the full Rust suite both ways to confirm no regressions**

```bash
cargo test --workspace -- --test-threads=1
```

Expected: all tests pass.

```bash
STINT_SKIP_KEYCHAIN_TESTS=1 cargo test --workspace -- --test-threads=1
```

Expected: all tests pass, one shows the "skipping:" message.

- [ ] **Step 7: Commit**

```bash
git add crates/stint-core/tests/config.rs
git commit -m "test(core): env-gate Keychain round-trip test for CI

Honors STINT_SKIP_KEYCHAIN_TESTS=1 by returning early. Local dev
runs the test unchanged; CI sets the env var because GitHub-hosted
macOS runners do not present a usable login keychain."
```

---

### Task 2: Add the CI workflow

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create the workflows directory**

```bash
mkdir -p .github/workflows
```

Expected: no output; verify with `ls -la .github/workflows`.

- [ ] **Step 2: Write `.github/workflows/ci.yml` verbatim**

```yaml
name: CI

on:
  pull_request:
    branches: [main]
  push:
    branches: [main]

# Cancel in-progress runs on the same PR when a new commit lands; let pushes
# to main run to completion so the badge / required-check status stays stable.
concurrency:
  group: ci-${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}

jobs:
  build:
    name: build
    runs-on: macos-14
    timeout-minutes: 30
    env:
      # See crates/stint-core/tests/config.rs — gates the one Keychain test.
      STINT_SKIP_KEYCHAIN_TESTS: "1"
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Install Rust 1.81
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: "1.81"
          components: rustfmt, clippy

      - name: Cache cargo registry, git db, and target/
        uses: Swatinem/rust-cache@v2

      - name: Install pnpm
        uses: pnpm/action-setup@v4
        with:
          version: 9

      - name: Install Node 20 (enables pnpm store cache)
        uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: pnpm
          cache-dependency-path: '**/pnpm-lock.yaml'

      - name: cargo fmt --check
        run: cargo fmt --all -- --check

      - name: cargo clippy (deny warnings)
        run: cargo clippy --workspace --all-targets -- -D warnings

      - name: cargo test
        run: cargo test --workspace -- --test-threads=1

      - name: pnpm install (root workspace)
        run: pnpm install --frozen-lockfile

      - name: pnpm install (ui)
        run: pnpm -C ui install --frozen-lockfile

      - name: pnpm typecheck
        run: pnpm -C ui typecheck

      - name: pnpm build
        run: pnpm -C ui build
```

**Cache key strategy (documented for reviewers):**

- `Swatinem/rust-cache@v2` keys on: rustc version + `Cargo.lock` content + workspace member set + job-name + OS. It restores `~/.cargo/registry`, `~/.cargo/git`, and `target/`, and cleans incremental build artifacts before save. No `key:` override needed — defaults are correct for this workspace.
- `actions/setup-node@v4` with `cache: pnpm` and `cache-dependency-path: '**/pnpm-lock.yaml'` keys on the SHA256 of every matching lockfile (root + `ui/`). Restores the pnpm content-addressable store (`~/Library/pnpm/store`).
- `pnpm install --frozen-lockfile` fails the run if any lockfile is out of date, which is desirable for a CI gate.

**Rust toolchain pin:** the workflow hardcodes `1.81` to match `rust-toolchain.toml`. This is a dual source of truth; bumping Rust requires editing both files. Acceptable for now (Phase 2.5 doesn't aim to solve toolchain plumbing). If it becomes annoying, a follow-up phase can read the channel from `rust-toolchain.toml` via a parsing step.

- [ ] **Step 3: Validate YAML syntax locally before pushing**

```bash
python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml')); print('ok')"
```

Expected: `ok`. If it errors, fix the YAML before committing — a syntax error on a remote run wastes the cold-cache minute and a roundtrip.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "chore(ci): add macOS GitHub Actions workflow for fmt/clippy/test/typecheck/build

Runs on every PR to main and every push to main. Single job on
macos-14 with Swatinem/rust-cache and pnpm store cache to keep
warm runs under 5 minutes. STINT_SKIP_KEYCHAIN_TESTS=1 set in
job env so the one Keychain-touching test is skipped on the
runner."
```

---

### Task 3: Push branch, open PR, iterate to green

This task is the only one with unbounded steps — getting a fresh GitHub Actions workflow green on the first try is rare. The point of opening as a draft PR is to use CI itself as the verification mechanism. Iterate with `fix(ci): ...` commits until the run is green.

- [ ] **Step 1: Push the branch**

```bash
git push -u origin phase-2.5
```

Expected: branch created on remote.

- [ ] **Step 2: Open a draft PR**

```bash
gh pr create --draft --base main --head phase-2.5 \
  --title "Phase 2.5: CI baseline" \
  --body "$(cat <<'EOF'
## Summary
- Add GitHub Actions CI workflow on macOS runner
- Env-gate the one Keychain-touching test for CI

## Test plan
- [ ] Workflow runs to completion on this PR
- [ ] Warm run (re-run on same SHA) completes under 5 min
- [ ] Branch protection added on main after merge (manual)
- [ ] Smoke test on a no-op commit (Task 6 of the plan)
EOF
)"
```

Expected: a PR URL is printed. Capture it for the next step.

- [ ] **Step 3: Watch the first (cold) run**

```bash
gh run watch
```

(If `gh run watch` requires a run ID, run `gh run list --branch phase-2.5 --limit 1` to find it first, then `gh run watch <id>`.)

Capture the cold-run wall-clock time from `gh run view <id>` for the Self-Review section.

- [ ] **Step 4: If the run failed, diagnose and iterate**

Common likely-failure modes and the `fix(ci): ...` for each:

| Failure | Diagnosis | Fix |
|---|---|---|
| `dtolnay/rust-toolchain@stable` cannot find toolchain `1.81` | Action tag interpretation | Try `dtolnay/rust-toolchain@1.81` (omit the `toolchain:` field); or pin to a specific patch like `1.81.0` |
| `pnpm install --frozen-lockfile` fails on root | Root lockfile drifted | Run `pnpm install` locally with no flags, commit the updated `pnpm-lock.yaml` |
| `pnpm install --frozen-lockfile` fails in `ui/` | `ui/pnpm-lock.yaml` drifted | Run `pnpm -C ui install` locally, commit |
| `cargo test` fails on a Keychain test even though env is set | Some other test path also touches Keychain | Grep `grep -rln keyring crates/stint-core/tests` and apply the same env-guard pattern, OR add `if std::env::var("STINT_SKIP_KEYCHAIN_TESTS").is_ok() { return; }` at the top of any extra test that needs it |
| `cargo clippy` fails on warnings that pass locally | New clippy lints triggered by stable channel drift OR by `--all-targets` (covers tests + benches that aren't in default `cargo check`) | Either fix the lint or, narrowly, add a targeted `#[allow]` — never blanket-disable |
| `pnpm -C ui build` fails | Vite plugin / TS error on a clean tree that's masked locally by stale `node_modules` | Reproduce locally with `rm -rf ui/node_modules && pnpm -C ui install && pnpm -C ui build`; fix the root cause |
| Step times out | Job timeout-minutes hit; very unlikely on a 30 min budget for cargo + pnpm | Investigate which step ran long; consider splitting jobs (out of scope here) |

For each iteration:

```bash
# Make the fix...
git add <files>
git commit -m "fix(ci): <one-line description of the fix>"
git push
gh run watch
```

- [ ] **Step 5: Once the run is green, trigger a warm re-run to capture warm timing**

```bash
gh run rerun <id>
gh run watch
```

The re-run reuses the cache populated by the cold run. Expected: under 5 min wall-clock. Capture the number; it goes in the Self-Review section.

If warm time exceeds 5 min, investigate caches:

```bash
# Inspect the run logs for cache hit/miss lines:
gh run view <id> --log | grep -i -E 'cache|restore|save'
```

- [ ] **Step 6: Mark the PR ready for review**

```bash
gh pr ready
```

Do NOT merge yet — Task 4 updates docs first, Task 5 sets up branch protection, Task 6 is the post-merge smoke test.

---

### Task 4: Update README and CLAUDE.md

**Files:**
- Modify: `README.md` (phase table)
- Modify: `CLAUDE.md` (phase table + branching-workflow note)

- [ ] **Step 1: Update `README.md` phase table**

Find the phase table row for 2.5 (currently `| 2.5 | CI baseline ... | planned |`) and change `planned` to the equivalent of the other shipped phases. Use the exact wording the README already uses for phases 1 and 2 — read those rows and match them.

- [ ] **Step 2: Update `CLAUDE.md` phase table the same way**

The table at the bottom of `CLAUDE.md` ("Where we are in the roadmap"). Change the 2.5 row from `planned` to `✅ shipped (`phase-2.5-complete`)`.

- [ ] **Step 3: Update `CLAUDE.md` "When you start work on a phase" section**

The current Step 6 reads: `Fast-forward main to the branch and push, then tag phase-N-complete.`

Branch protection (landing in Task 5) blocks direct pushes to `main`. Replace that step with:

```markdown
6. Open a PR from your phase branch to `main`. Wait for CI to go green.
   Merge via "Rebase and merge" in the GitHub UI (preserves linear
   history equivalent to a fast-forward). Then locally fetch, pull
   `main`, and tag `phase-N-complete` and push the tag.
```

- [ ] **Step 4: Add a "CI" entry to the `CLAUDE.md` "Gotchas / dev-environment notes" section**

Append (use the same formatting and tone as the existing gotchas):

```markdown
- **Keychain test is env-gated in CI.** `set_get_delete_round_trip` in
  `crates/stint-core/tests/config.rs` honors `STINT_SKIP_KEYCHAIN_TESTS=1`
  and returns early. CI sets it; local dev does not. If you add a new
  test that hits the real Keychain, copy the same three-line guard.
- **Rust toolchain pinned in two places.** `rust-toolchain.toml` pins
  `1.81` for local dev; `.github/workflows/ci.yml` pins `1.81` for CI.
  Bump both together.
```

- [ ] **Step 5: Verify nothing else needs updating**

```bash
grep -n "phase-2.5\|Phase 2.5\|phase 2.5" README.md CLAUDE.md AGENTS.md docs/superpowers/specs/2026-05-17-stint-design.md
```

Inspect each hit. The spec at §9 already describes Phase 2.5 in detail and doesn't need editing. AGENTS.md should mirror CLAUDE.md per CLAUDE.md's own statement; if AGENTS.md has the same phase table, update it identically.

- [ ] **Step 6: Commit**

```bash
git add README.md CLAUDE.md AGENTS.md
git commit -m "docs: mark Phase 2.5 (CI) shipped and update branching workflow

Updates the roadmap tables in README/CLAUDE/AGENTS to reflect
Phase 2.5 landing. Replaces the direct-push fast-forward step
with the PR + Rebase-and-merge flow that the new branch
protection rule requires. Adds two new gotchas: the env-gated
Keychain test and the dual Rust toolchain pin."
```

- [ ] **Step 7: Push and wait for CI green on the docs commit**

```bash
git push
gh run watch
```

Expected: green (docs-only commits should still pass the whole pipeline; verifies the workflow isn't accidentally tied to source-only changes).

---

### Task 5: Configure branch protection on `main`

This is **manual GitHub UI work**. Done by the repo admin (the user). The plan documents each setting exactly so it's reproducible.

- [ ] **Step 1: Navigate to branch protection settings**

Open the repo in a browser → `Settings` → `Branches` (left sidebar) → `Add branch ruleset` OR `Add classic branch protection rule`. The classic rule is simpler and sufficient — use it unless you have a reason for rulesets.

For classic: click `Add classic branch protection rule`. (If GitHub has phased out classic for new rules, use a ruleset; the same settings apply, just nested under "Rules" → "Branch protections".)

- [ ] **Step 2: Set the branch pattern**

`Branch name pattern:` enter `main`.

- [ ] **Step 3: Configure the checkboxes — turn ON exactly these**

- `Require a pull request before merging` — ON
  - `Required number of approvals before merging`: **0** (solo project)
  - `Dismiss stale pull request approvals when new commits are pushed`: leave off (no reviewers anyway)
  - `Require review from Code Owners`: OFF
  - `Require approval of the most recent reviewable push`: OFF
- `Require status checks to pass before merging` — ON
  - `Require branches to be up to date before merging`: ON
  - In the status checks search box, type `build` and select the **`build`** check (the job name from `ci.yml`). It only appears after the workflow has run at least once on a PR or push — Task 3's runs satisfy this.
- `Require conversation resolution before merging` — OFF (not using PR comments as gates)
- `Require signed commits` — OFF unless you already sign locally
- `Require linear history` — ON (matches the rebase-merge workflow)
- `Require deployments to succeed before merging` — OFF
- `Lock branch` — OFF
- `Do not allow bypassing the above settings` — OFF (you want admin bypass available)
- `Restrict who can push to matching branches` — OFF (you ARE the admin; the require-PR rule covers the gating)
- `Allow force pushes` — OFF
- `Allow deletions` — OFF

- [ ] **Step 4: Save the rule**

Click `Create` (or `Save changes` for an existing ruleset edit).

- [ ] **Step 5: Verify the rule blocks direct pushes**

From a local clone of `main`:

```bash
git checkout main
git pull
git commit --allow-empty -m "test: this should be rejected"
git push
```

Expected: push is rejected by remote with a message about branch protection. If it succeeds, the rule isn't configured — go back to Step 3.

Clean up the local test commit:

```bash
git reset --hard origin/main
```

---

### Task 6: Smoke test on a no-op commit

Verifies end-to-end: a fresh branch + empty commit + PR + CI green + protection-enforced merge.

- [ ] **Step 1: Branch off main**

```bash
git checkout main
git pull
git checkout -b ci-smoke
```

- [ ] **Step 2: No-op commit**

```bash
git commit --allow-empty -m "test: ci smoke — no-op to verify workflow on fresh PR"
```

- [ ] **Step 3: Push and open a PR**

```bash
git push -u origin ci-smoke
gh pr create --base main --head ci-smoke \
  --title "CI smoke test" \
  --body "No-op commit to verify the new CI gate works end-to-end on a fresh PR."
```

- [ ] **Step 4: Confirm protection blocks the merge until CI completes**

```bash
gh pr view --json mergeable,mergeStateStatus
```

Expected: `mergeStateStatus` is `BLOCKED` (or `BEHIND` if your branch is behind) while CI runs; not `CLEAN`.

- [ ] **Step 5: Watch the run**

```bash
gh run watch
```

Expected: green. Capture the timing — this is a warm-cache run on a no-op commit, so it should be near the best-case time captured in Task 3 Step 5.

- [ ] **Step 6: Confirm merge is now unblocked**

```bash
gh pr view --json mergeable,mergeStateStatus
```

Expected: `mergeStateStatus` is `CLEAN`.

- [ ] **Step 7: Merge via Rebase and merge**

```bash
gh pr merge --rebase --delete-branch
```

Expected: PR merged onto `main` with rebase strategy; branch deleted both locally and on remote.

- [ ] **Step 8: Verify CI runs on the resulting `main` push**

```bash
git checkout main
git pull
gh run list --branch main --limit 1
gh run watch
```

Expected: a run kicks off on the push-to-main and goes green. (If it does not run, the workflow's `on.push.branches` is misconfigured — fix and re-iterate.)

---

### Task 7: Merge phase-2.5 PR and tag

- [ ] **Step 1: Merge the Phase 2.5 PR**

Find the PR opened in Task 3:

```bash
gh pr list --state open --head phase-2.5
gh pr merge <number> --rebase --delete-branch
```

Expected: merged onto `main`.

- [ ] **Step 2: Confirm CI is green on `main`**

```bash
git checkout main
git pull
gh run list --branch main --limit 1
gh run watch  # or skip if the listed run is already completed-success
```

Expected: green.

- [ ] **Step 3: Tag and push**

```bash
git tag -a phase-2.5-complete -m "Phase 2.5: CI baseline (fmt, clippy, test, typecheck, build on macOS runner)"
git push origin phase-2.5-complete
```

- [ ] **Step 4: Verify the tag is visible**

```bash
gh release list 2>/dev/null  # tags may not be releases yet — fine
git ls-remote --tags origin | grep phase-2.5-complete
```

Expected: the tag appears on origin.

---

## Self-Review

After execution, the human reviewer (or executing agent) should walk this checklist:

**1. Spec coverage (§9 of the design spec):**

| Spec requirement | Plan task | Verified by |
|---|---|---|
| `cargo fmt --all -- --check` runs | Task 2 step in `ci.yml` | green run on PR |
| `cargo clippy --workspace --all-targets -- -D warnings` runs | Task 2 step in `ci.yml` | green run on PR |
| `cargo test --workspace -- --test-threads=1` runs | Task 2 step in `ci.yml` | green run on PR |
| `pnpm install --frozen-lockfile` (root + ui) runs | Task 2 two pnpm install steps | green run on PR |
| `pnpm -C ui typecheck` runs | Task 2 step in `ci.yml` | green run on PR |
| `pnpm -C ui build` runs | Task 2 step in `ci.yml` | green run on PR |
| Cargo cache (`~/.cargo/registry`, `target/`) | Task 2 — `Swatinem/rust-cache@v2` | warm-run time (Task 3 step 5) |
| pnpm cache (`~/.local/share/pnpm/store`) | Task 2 — `actions/setup-node@v4` with `cache: pnpm` | warm-run time (Task 3 step 5) |
| Warm runs under "a few minutes" | Task 3 step 5 + Task 6 step 5 | wall-clock observed |
| Required check on PRs merging to `main` | Task 5 branch protection | Task 6 step 4 (`BLOCKED` while pending) |
| Macos runner | Task 2 — `runs-on: macos-14` | run hostname in logs |
| One workflow file | Task 2 — `.github/workflows/ci.yml` | filesystem |

**2. Placeholder scan:**

Search this plan for the patterns `TODO`, `TBD`, `fill in`, `appropriate error handling`, `similar to Task`. There should be zero hits. If there are, fix before handing off.

**3. Type / name consistency:**

- The env var name `STINT_SKIP_KEYCHAIN_TESTS` appears in: Task 1 step 3 (test source), Task 2 step 2 (workflow `env`), Task 4 step 4 (gotcha). All three must spell it identically.
- The job name `build` appears in: Task 2 step 2 (`jobs.build`), Task 5 step 3 (status check selection). Both must match.
- The branch name `phase-2.5` appears throughout. The tag `phase-2.5-complete` appears in Task 7 step 3 and the README/CLAUDE updates.

**4. Captured numbers (fill in during execution):**

| Metric | Target | Observed |
|---|---|---|
| Cold-cache run wall-clock | — | _(fill in from Task 3 step 3)_ |
| Warm-cache run wall-clock | < 5 min | _(fill in from Task 3 step 5)_ |
| Smoke-test run wall-clock | near warm | _(fill in from Task 6 step 5)_ |

If warm is over 5 min, do not call the plan done — investigate cache hits in the run logs (`gh run view <id> --log | grep -i cache`) and iterate with a `fix(ci):` commit (e.g., the pnpm cache path may not match macOS reality and need an explicit `actions/cache` override).

---

## Execution Handoff

This plan covers a small, manual-heavy phase (the actual code change is ~10 lines; the rest is YAML, GitHub UI, and verification). Two execution options:

**1. Subagent-Driven (recommended)** — Fresh subagent per task, with the human (you) reviewing between tasks. Tasks 5 (branch protection UI), 6 (smoke test), and 7 (tag) naturally have human checkpoints; tasks 1–4 are mechanical.

**2. Inline Execution** — Execute tasks in this session with checkpoint pauses before Task 3 (first push), Task 5 (manual UI), and Task 7 (tag).

Either way, **stop and confirm with the human** before:
- Pushing the branch (Task 3 step 1)
- Performing the manual UI work (Task 5)
- Merging the PR (Task 7 step 1)
- Pushing the tag (Task 7 step 3)

Which approach?
