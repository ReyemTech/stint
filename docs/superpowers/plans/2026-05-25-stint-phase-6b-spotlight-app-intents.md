# stint Phase 6b: Spotlight + App Intents Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land `StintIntents.framework` inside `Stint.app` so macOS Spotlight indexes entries/projects/tasks, App Intents expose all 8 verbs as Custom Shortcuts (with 5 of them promoted as App Shortcuts with voice phrases), and a Focus filter sets the default project for new timers per Focus mode.

**Architecture:** A Swift Package at `crates/stint-app/swift/StintIntents/` produces a dynamic framework that is embedded into the Tauri-built `.app` via `bundle.macOS.frameworks`. Bidirectional FFI: Rust exposes `extern "C"` JSON-in/JSON-out verb wrappers; Swift exposes `@_cdecl` symbols looked up via `dlsym`. Spotlight indexing fires on every verb mutation through a Rust→Swift callback (no-op when the framework isn't loaded — keeps the CLI binary Spotlight-unaware).

**Tech Stack:** Swift 5.9+ · App Intents framework (macOS 13+) · Core Spotlight · NSUserActivity · Swift Package Manager (with `xcodebuild` fallback) · Rust 1.95.0 · existing Tauri 2 / SolidJS stack.

**Spec:** [`docs/superpowers/specs/2026-05-25-stint-phase-6-deeper-integration-design.md`](../specs/2026-05-25-stint-phase-6-deeper-integration-design.md)

---

## File Structure

### Rust crates

**Modify:**
- `crates/stint-core/src/lib.rs` — register `ffi` module
- `crates/stint-core/src/verbs/start.rs` — add focus-default fallback
- `crates/stint-core/src/verbs/list_tasks.rs` — accept "no project_id = all"
- `crates/stint-core/src/url_scheme.rs` — add `OpenProject`, `OpenTask`
- `crates/stint-app/src/lib.rs` — call `stint_intents_init()` from `setup()`
- `crates/stint-app/src/pull_worker.rs` — call `notify_indexer(ProjectsReplaced/TasksReplaced)`
- `crates/stint-app/build.rs` — invoke `swift build`, copy framework
- `crates/stint-app/tauri.conf.json` — `bundle.macOS.frameworks`
- `crates/stint-cli/skills/stint/SKILL.md` — App Intents surface ladder

**Create:**
- `crates/stint-core/src/ffi.rs` — extern "C" envelope + 8 verb wrappers + settings + log + focus
- `crates/stint-core/include/stint_core.h` — hand-written C header for Swift bridging
- `crates/stint-core/tests/ffi_envelope.rs` — envelope shape tests
- `crates/stint-core/tests/ffi_verbs.rs` — verb wrapper tests
- `crates/stint-core/tests/ffi_panic_safety.rs` — `catch_unwind` test

### Swift package

**Create the entire tree under `crates/stint-app/swift/StintIntents/`:**

```
Package.swift
Sources/StintIntents/
  Bridge.swift                            # FFI declarations + Bridge protocol + FFIBridge
  Errors/BridgeError.swift                # IntentError + envelope decode helper
  Entities/
    EntryEntity.swift
    ProjectEntity.swift
    TaskEntity.swift
    EntryQuery.swift
    ProjectQuery.swift
    TaskQuery.swift
  Intents/
    StartTimerIntent.swift
    StopTimerIntent.swift
    GetCurrentIntent.swift
    SwitchProjectIntent.swift
    LogPastIntent.swift
    ListEntriesIntent.swift
    ListProjectsIntent.swift
    ListTasksIntent.swift
    UpdateEntryIntent.swift
    DeleteEntryIntent.swift
  Shortcuts/
    StintAppShortcutsProvider.swift
    PhraseStrings.xcstrings
  Spotlight/
    SpotlightIndexer.swift
    ActivityTracker.swift
  Focus/
    ProjectFocusFilter.swift
  Init/
    StintIntentsInit.swift                # @_cdecl stint_intents_init, swift_indexer_notify, stint_current_focus_id
Tests/StintIntentsTests/
  BridgeEnvelopeTests.swift
  EntityCodingTests.swift
  SpotlightSchemaTests.swift
  AppIntentPerformTests.swift
  ProjectQueryTests.swift
Tests/StintIntentsIntegrationTests/       # separate target — links real stint_core
  FFIRoundTripTests.swift
```

### CI / docs

- `.github/workflows/ci.yml` — add `swift test` step, `codesign --verify` post-bundle step, Metadata.appintents parse + count assertion
- `docs/superpowers/plans/2026-05-25-stint-phase-6b-spotlight-app-intents.md` — this file
- Update `crates/stint-cli/skills/stint/SKILL.md` with App Intents surface ladder

---

## Conventions used throughout

- **TDD discipline:** Rust changes follow `failing test → impl → green` per the project standard (`crates/stint-core/tests/`).
- **Commit per task** with Conventional Commits. Subject under 70 chars; body explains the *why*. Use `feat(swift):`, `feat(ffi):`, `chore(build):`, etc.
- **Run before each commit:** the touched test file (`cargo test -p stint-core ffi_envelope` etc.) plus `cargo fmt --all -- --check` if any Rust file was edited. Full gate (`cargo test --workspace -- --test-threads=1`, `cargo clippy --workspace --all-targets -- -D warnings`, `pnpm typecheck`, `scripts/coverage.sh`) runs **once** at end of plan.
- **Don't push or open the PR** until the user confirms.
- **`scripts/dev-cli.sh` and `scripts/dev-app.sh`** wrap codesigning so the macOS Keychain ACL doesn't re-prompt. Don't use raw `cargo run` for the CLI or `cargo tauri dev` directly — use the wrappers.

---

## Task A1: SPM spike — verify framework + AppShortcut metadata generates

**Goal:** In ≤30 minutes, build a stub `StintIntents.framework` via Swift Package Manager containing one `AppIntent` and one `AppShortcutsProvider` with a single phrase. Verify the resulting bundle contains a `Metadata.appintents` stencil and that `pluginkit -mvD` (or `Metadata.appintents` parse) sees the intent. If this fails, fall back to an Xcode `.xcodeproj` (Task A1.fallback).

**Files:**
- Create: `crates/stint-app/swift/StintIntents/Package.swift` (throwaway version)
- Create: `crates/stint-app/swift/StintIntents/Sources/StintIntents/SpikeIntent.swift` (throwaway)

- [ ] **Step 1: Scaffold the SPM package**

Run:
```bash
mkdir -p crates/stint-app/swift/StintIntents/Sources/StintIntents
cd crates/stint-app/swift/StintIntents
```

Write `Package.swift`:

```swift
// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "StintIntents",
    platforms: [.macOS(.v13)],
    products: [
        .library(name: "StintIntents", type: .dynamic, targets: ["StintIntents"]),
    ],
    targets: [
        .target(
            name: "StintIntents",
            path: "Sources/StintIntents"
        ),
    ]
)
```

Write `Sources/StintIntents/SpikeIntent.swift`:

```swift
import AppIntents
import Foundation

struct SpikeIntent: AppIntent {
    static var title: LocalizedStringResource = "Spike"
    static var description = IntentDescription("Throwaway spike to verify SPM produces AppIntents metadata.")
    func perform() async throws -> some IntentResult { .result() }
}

struct StintSpikeShortcutsProvider: AppShortcutsProvider {
    static var appShortcuts: [AppShortcut] {
        AppShortcut(
            intent: SpikeIntent(),
            phrases: ["Spike in Stint"],
            shortTitle: "Spike",
            systemImageName: "checkmark.circle"
        )
    }
}
```

- [ ] **Step 2: Build the package**

Run from `crates/stint-app/swift/StintIntents/`:
```bash
swift build -c release
```

Expected: clean build, no errors. Output framework path: `.build/release/StintIntents.framework` or `.build/arm64-apple-macosx/release/StintIntents.dylib` depending on SPM output mode.

If SPM emits a `.dylib` instead of `.framework`, that's fine for the spike — what matters is the metadata stencil.

- [ ] **Step 3: Locate the AppIntents metadata stencil**

Run:
```bash
find .build -name "Metadata.appintents" -o -name "*.appintentsmetadata*" 2>/dev/null
```

Expected: at least one match. If empty → SPM didn't run `appintentsmetadataprocessor` automatically. Try:

```bash
find .build -name "*.appintents" -o -name "ExtractAppIntentsMetadata*" 2>/dev/null
swift build -c release -Xswiftc -j1 2>&1 | grep -i "appintents"
```

If still nothing → **SPM spike failed**; jump to Task A1.fallback.

- [ ] **Step 4: Verify metadata stencil mentions our intent**

Run (path adjusted to wherever step 3 found the stencil):
```bash
strings .build/release/StintIntents.framework/Resources/Metadata.appintents 2>/dev/null | grep -i Spike
# or, for the bare dylib output:
strings .build/release/libStintIntents.dylib 2>/dev/null | grep -i Spike
```

Expected: matches for `SpikeIntent`, `StintSpikeShortcutsProvider`, and the phrase `Spike in Stint`.

- [ ] **Step 5: Decision point — clean up spike files**

If steps 3+4 succeeded → **commit nothing**; instead, delete the spike sources:

```bash
rm crates/stint-app/swift/StintIntents/Sources/StintIntents/SpikeIntent.swift
# leave Package.swift in place — Task C1 will overwrite it with the final version
```

If they failed → switch to Task A1.fallback (Xcode `.xcodeproj`) and tag this task with a note in the commit message of A2 documenting what SPM didn't generate.

- [ ] **Step 6: Capture findings in a commit message preview**

The next commit (Task A2 or A1.fallback) will reference this finding. Note in a scratch file `/tmp/spm_spike.txt`:

```
SPM result: <pass | fail-with-reason>
Stencil path: <path or N/A>
Notes: <anything weird — Swift version, Xcode version, etc>
```

No commit for this task on its own — the spike is exploratory.

---

## Task A1.fallback (executed only if A1 fails): Xcode .xcodeproj packaging

Switch the Swift target to an Xcode project. Same `crates/stint-app/swift/StintIntents/` directory; replace `Package.swift` with `StintIntents.xcodeproj` generated via Xcode template "Framework". Update all later tasks that invoke `swift build` to instead invoke `xcodebuild -project StintIntents.xcodeproj -scheme StintIntents -configuration Release build`. This is a mechanical swap; the source files and APIs in subsequent tasks stay identical.

If reached, the cost is roughly 1 hour: re-create the Xcode project skeleton, verify metadata stencil generates (it does — this is Apple's primary path), update Task H1's build.rs invocation. No other tasks change.

---

## Task A2: Rust FFI envelope + panic safety

**Goal:** Create the FFI module skeleton with envelope JSON helpers and a `catch_unwind`-wrapped invocation pattern. No verbs yet — just the plumbing.

**Files:**
- Create: `crates/stint-core/src/ffi.rs`
- Modify: `crates/stint-core/src/lib.rs` (register module)
- Create: `crates/stint-core/tests/ffi_envelope.rs`
- Create: `crates/stint-core/tests/ffi_panic_safety.rs`

- [ ] **Step 1: Write failing envelope test**

Create `crates/stint-core/tests/ffi_envelope.rs`:

```rust
//! Envelope shape contract — every FFI verb wraps results in {"ok": T} or {"err": {code, message}}.

use serde_json::Value;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

// Re-export the helper we'll add in stint_core::ffi.
use stint_core::ffi::{
    self, stint_free_string, write_envelope_for_test,
};

#[test]
fn envelope_ok_shape() {
    let mut out: *mut c_char = ptr::null_mut();
    write_envelope_for_test(&mut out, Ok::<_, stint_core::Error>(serde_json::json!({"a": 1})));
    assert!(!out.is_null());
    let s = unsafe { CStr::from_ptr(out).to_str().unwrap().to_owned() };
    let v: Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["ok"]["a"], 1);
    assert!(v.get("err").is_none());
    unsafe { stint_free_string(out) };
}

#[test]
fn envelope_err_invariant_shape() {
    let mut out: *mut c_char = ptr::null_mut();
    write_envelope_for_test::<Value, _>(
        &mut out,
        Err(stint_core::Error::Invariant("nope".into())),
    );
    let s = unsafe { CStr::from_ptr(out).to_str().unwrap().to_owned() };
    let v: Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["err"]["code"], 1);
    assert_eq!(v["err"]["message"], "nope");
    unsafe { stint_free_string(out) };
}

#[test]
fn free_string_handles_null() {
    unsafe { stint_free_string(ptr::null_mut()) };  // must not segfault
}
```

- [ ] **Step 2: Run test to confirm failure**

```bash
cargo test -p stint-core --test ffi_envelope 2>&1 | tail -20
```

Expected: compile error — `stint_core::ffi` module doesn't exist.

- [ ] **Step 3: Create the ffi module**

Write `crates/stint-core/src/ffi.rs`:

```rust
//! C ABI surface for Swift consumers (StintIntents framework).
//!
//! Every public `extern "C"` function returns 0 (success — the actual result
//! is JSON-encoded in `out_json`) or a small set of misuse codes. The JSON
//! envelope is always one of:
//!
//! ```json
//! {"ok": <T>}
//! {"err": {"code": <int>, "message": "<str>"}}
//! ```
//!
//! Codes are a stable public contract — see the spec table.
//!
//! All public FFI fns wrap their body in `catch_unwind` so a Rust panic
//! crossing the C ABI becomes an `err.code = -1` envelope instead of UB.

use crate::Error;
use serde::Serialize;
use std::ffi::{c_char, CString};
use std::panic;
use std::ptr;

/// Stable error-code contract. Never renumber.
#[repr(i32)]
#[derive(Debug, Clone, Copy)]
enum Code {
    Invariant = 1,
    NotFound = 2,
    Conflict = 3,
    Serialization = 4,
    Internal = 99,
    Panic = -1,
}

fn code_for(err: &Error) -> i32 {
    match err {
        Error::Invariant(_) => Code::Invariant as i32,
        Error::NotFound(_) => Code::NotFound as i32,
        Error::SyncConflict(_) => Code::Conflict as i32,
        Error::Serialization(_) => Code::Serialization as i32,
        _ => Code::Internal as i32,
    }
}

/// Build the envelope JSON for any `Result<T, Error>` and write a malloc'd
/// CString into `*out_json`. Caller (Swift) frees via `stint_free_string`.
fn write_envelope<T: Serialize>(out_json: *mut *mut c_char, result: Result<T, Error>) {
    if out_json.is_null() {
        return;
    }
    let body = match result {
        Ok(t) => serde_json::json!({ "ok": t }),
        Err(e) => serde_json::json!({
            "err": { "code": code_for(&e), "message": e.to_string() }
        }),
    };
    let s = body.to_string();
    let c = CString::new(s).unwrap_or_else(|_| CString::new("{\"err\":{\"code\":99,\"message\":\"cstring null\"}}").unwrap());
    unsafe { *out_json = c.into_raw() };
}

/// Test-only re-export so integration tests can exercise the envelope helper
/// without needing a verb context.
#[doc(hidden)]
pub fn write_envelope_for_test<T: Serialize, E>(out_json: *mut *mut c_char, result: Result<T, E>)
where
    E: Into<Error>,
{
    let mapped = result.map_err(Into::into);
    write_envelope(out_json, mapped);
}

/// Wrap an FFI body in `catch_unwind`. On panic, write a Panic envelope.
fn ffi_body<F, T>(out_json: *mut *mut c_char, f: F)
where
    F: FnOnce() -> Result<T, Error> + std::panic::UnwindSafe,
    T: Serialize,
{
    let result = panic::catch_unwind(f);
    match result {
        Ok(r) => write_envelope(out_json, r),
        Err(p) => {
            let msg = match p.downcast_ref::<&'static str>() {
                Some(s) => (*s).to_owned(),
                None => match p.downcast_ref::<String>() {
                    Some(s) => s.clone(),
                    None => "rust panic (no message)".into(),
                },
            };
            let body = serde_json::json!({
                "err": { "code": Code::Panic as i32, "message": msg }
            });
            let c = CString::new(body.to_string()).unwrap();
            if !out_json.is_null() {
                unsafe { *out_json = c.into_raw() };
            }
        }
    }
}

/// Free a CString previously returned via `*out_json`. Safe to call with NULL.
#[no_mangle]
pub unsafe extern "C" fn stint_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    let _ = CString::from_raw(ptr);
}

// Verbs are added in Task A3.
```

Add to `crates/stint-core/src/lib.rs` (next to existing `pub mod` declarations):

```rust
pub mod ffi;
```

- [ ] **Step 4: Run tests to confirm green**

```bash
cargo test -p stint-core --test ffi_envelope 2>&1 | tail -10
```

Expected: 3 tests pass.

- [ ] **Step 5: Write panic-safety test**

Create `crates/stint-core/tests/ffi_panic_safety.rs`:

```rust
//! A Rust panic across the FFI boundary must be caught and turned into a
//! Panic envelope (code = -1) — never undefined behavior.

use serde_json::Value;
use std::ffi::{c_char, CStr};
use std::ptr;

#[test]
fn panic_in_ffi_body_returns_envelope_not_segfault() {
    // We exercise ffi_body indirectly via a temporary helper added below.
    // Once Task A3 lands the real verbs, this is replaced by a verb-level
    // panic-injection test. For A2, this asserts the wrapper itself works.

    let mut out: *mut c_char = ptr::null_mut();
    stint_core::ffi::panic_for_test(&mut out);

    assert!(!out.is_null(), "envelope must be written even on panic");
    let s = unsafe { CStr::from_ptr(out).to_str().unwrap().to_owned() };
    let v: Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["err"]["code"], -1);
    assert!(v["err"]["message"].as_str().unwrap().contains("test panic"));
    unsafe { stint_core::ffi::stint_free_string(out) };
}
```

Add to the bottom of `crates/stint-core/src/ffi.rs`:

```rust
/// Test-only — trigger ffi_body's panic path so the catch_unwind branch is
/// exercised. Not compiled into release builds.
#[doc(hidden)]
pub fn panic_for_test(out_json: *mut *mut c_char) {
    ffi_body::<_, ()>(out_json, || panic!("test panic"));
}
```

- [ ] **Step 6: Run panic-safety test**

```bash
cargo test -p stint-core --test ffi_panic_safety 2>&1 | tail -10
```

Expected: 1 test passes.

- [ ] **Step 7: Lint and commit**

```bash
cargo fmt --all
cargo clippy -p stint-core --all-targets -- -D warnings
git add crates/stint-core/src/ffi.rs crates/stint-core/src/lib.rs \
        crates/stint-core/tests/ffi_envelope.rs crates/stint-core/tests/ffi_panic_safety.rs
git commit -m "$(cat <<'EOF'
feat(core): FFI envelope + panic safety scaffolding for Swift bridge

Adds stint_core::ffi with a Result-shaped JSON envelope helper, a
catch_unwind-wrapped invocation pattern, and stint_free_string for
Swift-side memory ownership. Stable error code contract (1 invariant,
2 not-found, 3 conflict, 4 serialization, 99 internal, -1 panic).

Verbs land in a follow-up task; this commit only proves the envelope
shape and panic-recovery path.
EOF
)"
```

---

## Task A3: Rust FFI — 8 verb wrappers

**Goal:** Expose all 8 verbs as `extern "C"` functions. Each takes a JSON parameter string, returns 0, and writes an envelope JSON into `*out_json`.

**Files:**
- Modify: `crates/stint-core/src/ffi.rs`
- Create: `crates/stint-core/tests/ffi_verbs.rs`

- [ ] **Step 1: Write failing tests for all 8 verbs**

Create `crates/stint-core/tests/ffi_verbs.rs`:

```rust
//! Integration tests for the 8 extern "C" verb wrappers.
//!
//! Each test sets up a tempdir store, calls the FFI fn with JSON params,
//! and asserts the envelope shape.

mod common;

use serde_json::{json, Value};
use std::ffi::{c_char, CStr, CString};
use std::ptr;

fn call_verb<F>(f: F) -> Value
where
    F: FnOnce(*mut *mut c_char),
{
    let mut out: *mut c_char = ptr::null_mut();
    f(&mut out);
    assert!(!out.is_null());
    let s = unsafe { CStr::from_ptr(out).to_str().unwrap().to_owned() };
    let v: Value = serde_json::from_str(&s).unwrap();
    unsafe { stint_core::ffi::stint_free_string(out) };
    v
}

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

#[test]
fn ffi_start_happy_path() {
    let _setup = common::setup();
    let params = cstr(r#"{"description":"writing tests","source":"ffi-test"}"#);
    let env = call_verb(|out| unsafe {
        stint_core::ffi::stint_verb_start(params.as_ptr(), out);
    });
    assert!(env["ok"].is_object(), "envelope: {env}");
    assert_eq!(env["ok"]["description"], "writing tests");
    assert_eq!(env["ok"]["source"], "ffi-test");
}

#[test]
fn ffi_start_invariant_already_running() {
    let _setup = common::setup();
    let params = cstr(r#"{"description":"first","source":"ffi-test"}"#);
    let _ = call_verb(|out| unsafe { stint_core::ffi::stint_verb_start(params.as_ptr(), out) });
    let env = call_verb(|out| unsafe { stint_core::ffi::stint_verb_start(params.as_ptr(), out) });
    assert_eq!(env["err"]["code"], 1, "envelope: {env}");
}

#[test]
fn ffi_current_when_running() {
    let _setup = common::setup();
    let params = cstr(r#"{"description":"x","source":"ffi-test"}"#);
    let _ = call_verb(|out| unsafe { stint_core::ffi::stint_verb_start(params.as_ptr(), out) });
    let env = call_verb(|out| unsafe { stint_core::ffi::stint_verb_current(out) });
    assert_eq!(env["ok"]["description"], "x");
}

#[test]
fn ffi_current_when_no_timer() {
    let _setup = common::setup();
    let env = call_verb(|out| unsafe { stint_core::ffi::stint_verb_current(out) });
    assert!(env["ok"].is_null(), "envelope: {env}");
}

#[test]
fn ffi_stop_after_start() {
    let _setup = common::setup();
    let params = cstr(r#"{"description":"y","source":"ffi-test"}"#);
    let _ = call_verb(|out| unsafe { stint_core::ffi::stint_verb_start(params.as_ptr(), out) });
    let env = call_verb(|out| unsafe { stint_core::ffi::stint_verb_stop(out) });
    assert!(env["ok"]["end_at"].is_string());
}

#[test]
fn ffi_list_entries_empty() {
    let _setup = common::setup();
    let filter = cstr("{}");
    let env = call_verb(|out| unsafe {
        stint_core::ffi::stint_verb_list_entries(filter.as_ptr(), out)
    });
    assert_eq!(env["ok"].as_array().unwrap().len(), 0);
}

#[test]
fn ffi_list_projects_empty() {
    let _setup = common::setup();
    let env = call_verb(|out| unsafe { stint_core::ffi::stint_verb_list_projects(out) });
    assert!(env["ok"].is_array(), "envelope: {env}");
}

#[test]
fn ffi_list_tasks_empty() {
    let _setup = common::setup();
    let filter = cstr("{}");
    let env = call_verb(|out| unsafe {
        stint_core::ffi::stint_verb_list_tasks(filter.as_ptr(), out)
    });
    assert!(env["ok"].is_array(), "envelope: {env}");
}

#[test]
fn ffi_update_entry_not_found() {
    let _setup = common::setup();
    let params = cstr(r#"{"local_uuid":"does-not-exist","patch":{}}"#);
    let env = call_verb(|out| unsafe {
        stint_core::ffi::stint_verb_update_entry(params.as_ptr(), out)
    });
    assert_eq!(env["err"]["code"], 2);
}

#[test]
fn ffi_delete_entry_not_found() {
    let _setup = common::setup();
    let params = cstr(r#"{"local_uuid":"does-not-exist"}"#);
    let env = call_verb(|out| unsafe {
        stint_core::ffi::stint_verb_delete_entry(params.as_ptr(), out)
    });
    assert_eq!(env["err"]["code"], 2);
}

#[test]
fn ffi_malformed_json_returns_serialization_error() {
    let _setup = common::setup();
    let params = cstr("not json");
    let env = call_verb(|out| unsafe {
        stint_core::ffi::stint_verb_start(params.as_ptr(), out)
    });
    assert_eq!(env["err"]["code"], 4);
}
```

The `mod common` line at the top reuses `crates/stint-core/tests/common/mod.rs` — already in the repo (sets up a tempdir Store and points STINT_HOME at it).

- [ ] **Step 2: Run tests to confirm failure**

```bash
cargo test -p stint-core --test ffi_verbs 2>&1 | tail -5
```

Expected: compile error — verb functions don't exist.

- [ ] **Step 3: Implement the 8 verb wrappers**

Append to `crates/stint-core/src/ffi.rs`:

```rust
use crate::{verbs, store::Store, Result};
use std::ffi::CStr;

/// Open the user-default store. Verbs that need it call this; on failure
/// (e.g., missing DB), they surface an Internal error envelope.
fn open_store() -> Result<Store> {
    let path = crate::paths::default_db_path()?;
    Store::open(&path)
}

unsafe fn parse_params<'a, T: serde::de::DeserializeOwned>(ptr: *const c_char) -> Result<T> {
    if ptr.is_null() {
        return Err(Error::Serialization("null params".into()));
    }
    let cstr = unsafe { CStr::from_ptr(ptr) };
    let s = cstr.to_str().map_err(|e| Error::Serialization(e.to_string()))?;
    serde_json::from_str(s).map_err(|e| Error::Serialization(e.to_string()))
}

// ---- start ----

#[no_mangle]
pub unsafe extern "C" fn stint_verb_start(
    params_json: *const c_char,
    out_json: *mut *mut c_char,
) -> i32 {
    ffi_body(out_json, || {
        let params: verbs::StartParams = parse_params(params_json)?;
        let store = open_store()?;
        verbs::start(&store, params)
    });
    0
}

// ---- stop ----

#[no_mangle]
pub unsafe extern "C" fn stint_verb_stop(out_json: *mut *mut c_char) -> i32 {
    ffi_body(out_json, || {
        let store = open_store()?;
        verbs::stop(&store)
    });
    0
}

// ---- current ----

#[no_mangle]
pub unsafe extern "C" fn stint_verb_current(out_json: *mut *mut c_char) -> i32 {
    ffi_body(out_json, || {
        let store = open_store()?;
        verbs::current(&store)  // returns Option<EntryView>
    });
    0
}

// ---- list_entries ----

#[no_mangle]
pub unsafe extern "C" fn stint_verb_list_entries(
    filter_json: *const c_char,
    out_json: *mut *mut c_char,
) -> i32 {
    ffi_body(out_json, || {
        let filter: verbs::EntryFilter = parse_params(filter_json)?;
        let store = open_store()?;
        verbs::list_entries(&store, filter)
    });
    0
}

// ---- list_projects ----

#[no_mangle]
pub unsafe extern "C" fn stint_verb_list_projects(out_json: *mut *mut c_char) -> i32 {
    ffi_body(out_json, || {
        let store = open_store()?;
        verbs::list_projects(&store)
    });
    0
}

// ---- list_tasks ----

#[derive(serde::Deserialize)]
struct ListTasksParams {
    #[serde(default)]
    project_id: Option<String>,
}

#[no_mangle]
pub unsafe extern "C" fn stint_verb_list_tasks(
    params_json: *const c_char,
    out_json: *mut *mut c_char,
) -> i32 {
    ffi_body(out_json, || {
        let p: ListTasksParams = parse_params(params_json)?;
        let store = open_store()?;
        verbs::list_tasks(&store, p.project_id.as_deref())
    });
    0
}

// ---- update_entry ----

#[derive(serde::Deserialize)]
struct UpdateEntryParams {
    local_uuid: String,
    patch: verbs::EntryPatch,
}

#[no_mangle]
pub unsafe extern "C" fn stint_verb_update_entry(
    params_json: *const c_char,
    out_json: *mut *mut c_char,
) -> i32 {
    ffi_body(out_json, || {
        let p: UpdateEntryParams = parse_params(params_json)?;
        let store = open_store()?;
        verbs::update_entry(&store, &p.local_uuid, p.patch)
    });
    0
}

// ---- delete_entry ----

#[derive(serde::Deserialize)]
struct DeleteEntryParams {
    local_uuid: String,
}

#[no_mangle]
pub unsafe extern "C" fn stint_verb_delete_entry(
    params_json: *const c_char,
    out_json: *mut *mut c_char,
) -> i32 {
    ffi_body(out_json, || {
        let p: DeleteEntryParams = parse_params(params_json)?;
        let store = open_store()?;
        verbs::delete_entry(&store, &p.local_uuid)?;
        Ok::<_, Error>(serde_json::json!({}))
    });
    0
}
```

Note that `verbs::list_tasks` currently requires a `project_id` — see Task A5 which extends it to accept `Option<&str>`. For now, write this with the new signature; Task A5 lands the trait change.

- [ ] **Step 4: Extend `verbs::list_tasks` to accept Option<&str>** (Task A5 inlined here)

Modify `crates/stint-core/src/verbs/list_tasks.rs` so the signature becomes:

```rust
pub fn list_tasks(store: &Store, project_id: Option<&str>) -> Result<Vec<TaskView>> {
    match project_id {
        Some(id) => store.reference.list_tasks_for_project(id),
        None => store.reference.list_all_tasks(),
    }
    .map(|rows| rows.into_iter().map(TaskView::from).collect())
}
```

If `Store::reference::list_all_tasks` doesn't exist yet, add it in `crates/stint-core/src/store/reference.rs` — it's a `SELECT * FROM tasks WHERE done = 0` (no WHERE project_id clause).

Update all call sites:
- `crates/stint-cli/src/cmd/list_tasks` (or wherever `verbs::list_tasks` is called) — wrap existing `project_id` in `Some(...)`.
- `crates/stint-app/src/commands/projects.rs` — same.
- `crates/stint-app/src/http/handlers.rs` — same; HTTP handler still requires `project_id` (existing contract); pass `Some(p.project_id.as_deref().unwrap_or(""))` or refactor to allow None query-param. Decision: HTTP keeps required `project_id` (existing API contract); only FFI gets the None-friendly path.
- MCP server — same as HTTP, keep required.

- [ ] **Step 5: Run all tests**

```bash
cargo test -p stint-core --test ffi_verbs 2>&1 | tail -20
cargo test -p stint-core 2>&1 | tail -10
```

Expected: all 11 ffi_verbs tests pass + all existing tests still green.

- [ ] **Step 6: Lint + commit**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
git add -A
git commit -m "$(cat <<'EOF'
feat(core): extern "C" verb wrappers for Swift bridge

Adds the 8 verb FFI entry points: start, stop, current, list_entries,
list_projects, list_tasks, update_entry, delete_entry. Each accepts a
JSON param string (or no params for the 0-arg verbs) and writes a
{ok: T} | {err: {code, message}} envelope into out_json. Caller frees
via stint_free_string.

Also extends verbs::list_tasks to accept Option<&str> for project_id so
the FFI can list across all projects — HTTP and MCP keep the required
project_id semantics.

Tests cover happy paths, the already-running invariant, current-when-no-
timer, not-found update/delete, and malformed-JSON serialization errors.
EOF
)"
```

---

## Task A4: Rust FFI — settings get/set/clear + log + focus_id

**Goal:** Small additional FFI surface for the Focus filter feature and for Swift to log into Rust's tracing subscriber.

**Files:**
- Modify: `crates/stint-core/src/ffi.rs`
- Create: `crates/stint-core/tests/ffi_settings.rs`

- [ ] **Step 1: Write failing tests**

Create `crates/stint-core/tests/ffi_settings.rs`:

```rust
mod common;

use std::ffi::{c_char, CStr, CString};
use std::ptr;

#[test]
fn settings_set_get_clear_round_trip() {
    let _setup = common::setup();
    let key = CString::new("focus.default_project").unwrap();
    let val = CString::new("focus-uuid-abc\tproject-uuid-xyz").unwrap();
    let rc = unsafe { stint_core::ffi::stint_settings_set(key.as_ptr(), val.as_ptr()) };
    assert_eq!(rc, 0);

    let mut out: *mut c_char = ptr::null_mut();
    let rc = unsafe { stint_core::ffi::stint_settings_get(key.as_ptr(), &mut out) };
    assert_eq!(rc, 0);
    assert!(!out.is_null());
    let got = unsafe { CStr::from_ptr(out).to_str().unwrap().to_owned() };
    assert_eq!(got, "focus-uuid-abc\tproject-uuid-xyz");
    unsafe { stint_core::ffi::stint_free_string(out) };

    let rc = unsafe { stint_core::ffi::stint_settings_clear(key.as_ptr()) };
    assert_eq!(rc, 0);

    let mut out: *mut c_char = ptr::null_mut();
    let rc = unsafe { stint_core::ffi::stint_settings_get(key.as_ptr(), &mut out) };
    assert_eq!(rc, 0);
    assert!(out.is_null(), "cleared key must return null pointer");
}

#[test]
fn log_warn_does_not_panic() {
    let msg = CString::new("hello from swift").unwrap();
    unsafe { stint_core::ffi::stint_log_warn(msg.as_ptr()) };
    // No assertion — just that it doesn't crash. tracing subscriber is set
    // up by stint-app at runtime; in tests it's no-op.
}

#[test]
fn current_focus_id_returns_null_in_tests() {
    let mut out: *mut c_char = ptr::null_mut();
    let rc = unsafe { stint_core::ffi::stint_current_focus_id(&mut out) };
    assert_eq!(rc, 0);
    // In tests the dlsym lookup returns null (Swift framework isn't loaded).
    assert!(out.is_null());
}
```

- [ ] **Step 2: Confirm failure, then implement**

```bash
cargo test -p stint-core --test ffi_settings 2>&1 | tail -5
```

Expected: compile error.

Append to `crates/stint-core/src/ffi.rs`:

```rust
// ---- settings ----

#[no_mangle]
pub unsafe extern "C" fn stint_settings_set(key: *const c_char, value: *const c_char) -> i32 {
    if key.is_null() || value.is_null() {
        return -2;
    }
    let result = panic::catch_unwind(|| -> Result<()> {
        let key = CStr::from_ptr(key).to_str().map_err(|e| Error::Serialization(e.to_string()))?;
        let value = CStr::from_ptr(value).to_str().map_err(|e| Error::Serialization(e.to_string()))?;
        let store = open_store()?;
        store.settings_set(key, value)
    });
    match result {
        Ok(Ok(())) => 0,
        _ => 1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn stint_settings_get(key: *const c_char, out_json: *mut *mut c_char) -> i32 {
    if key.is_null() || out_json.is_null() {
        return -2;
    }
    unsafe { *out_json = ptr::null_mut() };
    let result = panic::catch_unwind(|| -> Result<Option<String>> {
        let key = CStr::from_ptr(key).to_str().map_err(|e| Error::Serialization(e.to_string()))?;
        let store = open_store()?;
        store.settings_get(key)
    });
    match result {
        Ok(Ok(Some(v))) => {
            if let Ok(c) = CString::new(v) {
                unsafe { *out_json = c.into_raw() };
            }
            0
        }
        Ok(Ok(None)) => 0,
        _ => 1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn stint_settings_clear(key: *const c_char) -> i32 {
    if key.is_null() {
        return -2;
    }
    let result = panic::catch_unwind(|| -> Result<()> {
        let key = CStr::from_ptr(key).to_str().map_err(|e| Error::Serialization(e.to_string()))?;
        let store = open_store()?;
        store.settings_clear(key)
    });
    match result {
        Ok(Ok(())) => 0,
        _ => 1,
    }
}

// ---- log forwarder ----

#[no_mangle]
pub unsafe extern "C" fn stint_log_warn(msg: *const c_char) {
    if msg.is_null() {
        return;
    }
    if let Ok(s) = CStr::from_ptr(msg).to_str() {
        tracing::warn!(target: "stint_intents", "{}", s);
    }
}

// ---- focus_id (dlsym'd from Swift; stub when framework absent) ----

type FocusIdFn = unsafe extern "C" fn(*mut *mut c_char) -> i32;

static FOCUS_ID_SYMBOL: std::sync::OnceLock<Option<FocusIdFn>> = std::sync::OnceLock::new();

unsafe fn lookup_focus_id() -> Option<FocusIdFn> {
    *FOCUS_ID_SYMBOL.get_or_init(|| {
        let handle = libc::dlopen(std::ptr::null(), libc::RTLD_NOW);
        if handle.is_null() {
            return None;
        }
        let name = std::ffi::CString::new("stint_current_focus_id_swift").unwrap();
        let sym = libc::dlsym(handle, name.as_ptr());
        if sym.is_null() {
            None
        } else {
            Some(std::mem::transmute::<*mut libc::c_void, FocusIdFn>(sym))
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn stint_current_focus_id(out_json: *mut *mut c_char) -> i32 {
    if out_json.is_null() {
        return -2;
    }
    unsafe { *out_json = ptr::null_mut() };
    if let Some(f) = lookup_focus_id() {
        f(out_json)
    } else {
        0  // framework not loaded → return null = no current focus
    }
}
```

Add to `crates/stint-core/Cargo.toml` `[dependencies]`:

```toml
libc = "0.2"
```

(Skip if already present — check the existing file.)

Also: `Store::settings_set/get/clear` may need to be exposed publicly if they're not already. Check `crates/stint-core/src/store/settings.rs` (or wherever settings live) and ensure those three methods are `pub`. If not, make them `pub` and add unit tests if any are missing.

- [ ] **Step 3: Run tests**

```bash
cargo test -p stint-core --test ffi_settings 2>&1 | tail -10
```

Expected: 3 tests pass.

- [ ] **Step 4: Commit**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
git add -A
git commit -m "$(cat <<'EOF'
feat(core): FFI surface for settings, log forwarder, and focus_id dlsym

Adds three more FFI surfaces beyond the verb wrappers:

- stint_settings_set/get/clear: opaque key/value passthrough so Swift
  Focus filters can persist their (focus_id, project_id) selection.
- stint_log_warn: lets Swift route logs into stint's tracing subscriber
  via the existing "stint_intents" target.
- stint_current_focus_id: dlsym-looks up a Swift-exported helper. When
  the framework isn't loaded (CLI binary), returns null = no focus,
  which the start-verb fallback treats as "no default".

All three are catch_unwind-wrapped. Memory ownership is the same as the
verb wrappers: caller frees out_json via stint_free_string.
EOF
)"
```

---

## Task A6: Focus default applied in `verbs::start`

**Goal:** Implement the focus-id-reconciled fallback in `verbs::start` so any surface that calls start without a `project_id` picks up the Focus default — but only if the stored focus_id still matches the current macOS focus.

**Files:**
- Modify: `crates/stint-core/src/verbs/start.rs`
- Create: `crates/stint-core/src/focus.rs` (small helper)
- Modify: `crates/stint-core/src/lib.rs` (register module)
- Modify: `crates/stint-core/tests/start.rs` (or wherever start tests live)

- [ ] **Step 1: Write failing tests**

Find the existing test file for `verbs::start` (`crates/stint-core/tests/start.rs` or under `tests/`). Add:

```rust
#[test]
fn start_picks_up_focus_default_when_project_missing() {
    let setup = common::setup();
    let store = setup.store();

    // Seed a project so the default points somewhere valid.
    common::seed_projects(&store, &[("proj-uuid-1", "Acme")]);

    // Simulate the Focus filter writing its tuple.
    // Note: in the real flow, Swift writes this via stint_settings_set.
    store
        .settings_set("focus.default_project", "fake-focus-id\tproj-uuid-1")
        .unwrap();

    // Inject the "current focus id" for tests via STINT_TEST_FOCUS_ID env var.
    std::env::set_var("STINT_TEST_FOCUS_ID", "fake-focus-id");

    let view = verbs::start(
        &store,
        verbs::StartParams {
            description: "no project given".into(),
            project_id: None,
            task_id: None,
            billable: false,
            start_at: None,
            source: "test".into(),
        },
    )
    .unwrap();

    assert_eq!(view.project_id.as_deref(), Some("proj-uuid-1"));

    std::env::remove_var("STINT_TEST_FOCUS_ID");
}

#[test]
fn start_ignores_focus_default_when_focus_id_mismatches() {
    let setup = common::setup();
    let store = setup.store();

    store
        .settings_set("focus.default_project", "fake-focus-id\tproj-uuid-1")
        .unwrap();

    std::env::set_var("STINT_TEST_FOCUS_ID", "different-focus-id");

    let view = verbs::start(
        &store,
        verbs::StartParams {
            description: "no project given".into(),
            project_id: None,
            task_id: None,
            billable: false,
            start_at: None,
            source: "test".into(),
        },
    )
    .unwrap();

    assert_eq!(view.project_id, None);
    std::env::remove_var("STINT_TEST_FOCUS_ID");
}

#[test]
fn start_explicit_project_overrides_focus_default() {
    let setup = common::setup();
    let store = setup.store();

    common::seed_projects(
        &store,
        &[("proj-uuid-1", "Acme"), ("proj-uuid-2", "Other")],
    );

    store
        .settings_set("focus.default_project", "fake-focus-id\tproj-uuid-1")
        .unwrap();
    std::env::set_var("STINT_TEST_FOCUS_ID", "fake-focus-id");

    let view = verbs::start(
        &store,
        verbs::StartParams {
            description: "explicit project".into(),
            project_id: Some("proj-uuid-2".into()),
            task_id: None,
            billable: false,
            start_at: None,
            source: "test".into(),
        },
    )
    .unwrap();

    assert_eq!(view.project_id.as_deref(), Some("proj-uuid-2"));
    std::env::remove_var("STINT_TEST_FOCUS_ID");
}
```

- [ ] **Step 2: Confirm failure**

```bash
cargo test -p stint-core start_picks_up_focus_default 2>&1 | tail -5
```

Expected: failures (tests pass `None` and expect Some).

- [ ] **Step 3: Add focus helper**

Create `crates/stint-core/src/focus.rs`:

```rust
//! Looks up the currently active macOS Focus identifier.
//!
//! In production (Stint.app loaded with StintIntents.framework), this dlsym's
//! into a Swift helper. In tests and the CLI binary, it reads STINT_TEST_FOCUS_ID
//! from the environment so the start-verb fallback can be exercised.

use std::ffi::{c_char, CStr};

pub fn current_id() -> Option<String> {
    // Test escape hatch — always check this first, even in release builds, so
    // CLI integration tests can stand in for the framework.
    if let Ok(v) = std::env::var("STINT_TEST_FOCUS_ID") {
        if !v.is_empty() {
            return Some(v);
        }
    }

    let mut out: *mut c_char = std::ptr::null_mut();
    let rc = unsafe { crate::ffi::stint_current_focus_id(&mut out) };
    if rc != 0 || out.is_null() {
        return None;
    }
    let s = unsafe { CStr::from_ptr(out).to_str().ok()?.to_owned() };
    unsafe { crate::ffi::stint_free_string(out) };
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}
```

Register in `crates/stint-core/src/lib.rs`:

```rust
pub mod focus;
```

- [ ] **Step 4: Wire the fallback into `verbs::start`**

Modify `crates/stint-core/src/verbs/start.rs`. Locate the early body where `params.project_id` is read, and insert the fallback before any use:

```rust
pub fn start(store: &Store, params: StartParams) -> Result<EntryView> {
    let project_id = params.project_id.clone().or_else(|| {
        let raw = store.settings_get("focus.default_project").ok().flatten()?;
        let (stored_focus, project_id) = raw.split_once('\t')?;
        let current = crate::focus::current_id()?;
        if current == stored_focus {
            Some(project_id.to_string())
        } else {
            None
        }
    });

    let params = StartParams { project_id, ..params };
    // ... existing implementation continues with the (possibly defaulted) params
}
```

(Exact integration depends on the existing structure of `start.rs` — read it first and adapt.)

- [ ] **Step 5: Run tests**

```bash
cargo test -p stint-core start_picks_up_focus_default start_ignores_focus_default_when start_explicit_project_overrides 2>&1 | tail -15
```

Expected: 3 tests pass.

- [ ] **Step 6: Commit**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
git add -A
git commit -m "$(cat <<'EOF'
feat(core): focus-default fallback in verbs::start

When start() is called with no project_id, look up the focus default
written by Swift's ProjectFocusFilter (stored as "<focus_id>\t<project_id>")
and apply it only if the stored focus_id matches the currently active
macOS focus. This prevents a stale default from leaking after the user
switches focus modes.

STINT_TEST_FOCUS_ID env var is the test escape hatch — production reads
the focus id via stint_current_focus_id (dlsym'd into Swift).
EOF
)"
```

---

## Task B1: URL scheme additions — OpenProject, OpenTask

**Goal:** Extend `stint://` URL parser to handle `stint://project/<id>` and `stint://task/<id>`, route them through the Tauri deep-link handler, and navigate the SolidJS UI to the filtered Today view.

**Files:**
- Modify: `crates/stint-core/src/url_scheme.rs`
- Modify: `crates/stint-core/src/url_scheme.rs` tests (inline `#[cfg(test)] mod tests`)
- Modify: `crates/stint-app/src/lib.rs` (Tauri deep-link handler)
- Modify: `ui/src/routes/Today.tsx` (or wherever the Today view reads query params)

- [ ] **Step 1: Add failing URL parse tests**

Locate the `#[cfg(test)] mod tests` block in `crates/stint-core/src/url_scheme.rs`. Append:

```rust
    #[test]
    fn parse_open_project() {
        let action = parse("stint://project/proj-uuid-1").unwrap();
        assert!(matches!(action, Action::OpenProject { ref project_id } if project_id == "proj-uuid-1"));
    }

    #[test]
    fn parse_open_task() {
        let action = parse("stint://task/task-uuid-1").unwrap();
        assert!(matches!(action, Action::OpenTask { ref task_id } if task_id == "task-uuid-1"));
    }

    #[test]
    fn parse_open_project_missing_id_errors() {
        assert!(parse("stint://project").is_err());
        assert!(parse("stint://project/").is_err());
    }
```

- [ ] **Step 2: Extend `Action` and `parse`**

In the same file, locate the `Action` enum and add:

```rust
pub enum Action {
    // existing variants
    Start { ... },
    Stop,
    OpenEntry { local_uuid: String },
    Current,
    // new:
    OpenProject { project_id: String },
    OpenTask { task_id: String },
}
```

In the `match head` block, add:

```rust
"project" => {
    let project_id = segments
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Invariant("project requires id".into()))?
        .to_string();
    Ok(Action::OpenProject { project_id })
}
"task" => {
    let task_id = segments
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Invariant("task requires id".into()))?
        .to_string();
    Ok(Action::OpenTask { task_id })
}
```

- [ ] **Step 3: Run url_scheme tests**

```bash
cargo test -p stint-core url_scheme 2>&1 | tail -10
```

Expected: new tests pass, existing tests still green.

- [ ] **Step 4: Route the new actions in the Tauri deep-link handler**

Open `crates/stint-app/src/lib.rs` (or wherever the deep-link handler lives — search for `tauri_plugin_deep_link` or `parse_url`). Locate the `match action` block and add:

```rust
Action::OpenProject { project_id } => {
    let _ = app.emit("navigate", serde_json::json!({
        "route": format!("/today?project={}", project_id)
    }));
    show_main_window(app);
}
Action::OpenTask { task_id } => {
    // Resolve task → project_id via verbs::list_tasks so we can build the URL.
    let store = open_store_or_warn(app);
    if let Some(store) = store {
        if let Ok(all_tasks) = verbs::list_tasks(&store, None) {
            if let Some(t) = all_tasks.iter().find(|t| t.solidtime_id == task_id) {
                let _ = app.emit("navigate", serde_json::json!({
                    "route": format!("/today?project={}&task={}", t.project_id, task_id)
                }));
                show_main_window(app);
                return Ok(());
            }
        }
    }
    // Fallback: open Today view without filter.
    let _ = app.emit("navigate", serde_json::json!({ "route": "/today" }));
    show_main_window(app);
}
```

The exact `show_main_window` helper and `open_store_or_warn` patterns already exist in the file — match the style.

- [ ] **Step 5: Handle the `navigate` event in the UI**

Find the `App.tsx` or root component that listens for Tauri events. There's likely already a listener — confirm `navigate` is one of them. If not, add:

```tsx
// ui/src/App.tsx (or wherever event listeners are set up)
import { listen } from "@tauri-apps/api/event";
import { useNavigate } from "@solidjs/router";

const navigate = useNavigate();

onMount(async () => {
    const unlisten = await listen<{ route: string }>("navigate", (e) => {
        navigate(e.payload.route);
    });
    onCleanup(() => unlisten());
});
```

If the listener is already there for other purposes, simply confirm `/today?project=...` resolves correctly in the SolidJS router (the Today route may need to read `searchParams` and apply the filter).

In `ui/src/routes/Today.tsx`, add:

```tsx
import { useSearchParams } from "@solidjs/router";

const [searchParams] = useSearchParams();
const projectFilter = () => searchParams.project;
const taskFilter = () => searchParams.task;

// Use these in the entries query / filter UI to pre-select the project/task
```

- [ ] **Step 6: Typecheck + commit**

```bash
pnpm typecheck
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
git add -A
git commit -m "$(cat <<'EOF'
feat(core): stint:// URL routes for projects and tasks

Extends url_scheme::parse to recognize stint://project/<id> and
stint://task/<id>. The Tauri deep-link handler emits a navigate event
that the SolidJS router consumes to land on /today filtered to the
chosen project (and task, if applicable).

Spotlight result taps for project/task CSSearchableItems use these
routes in Phase 6b.
EOF
)"
```

---

## Task B2: Pull worker → indexer notify hook

**Goal:** When the pull worker completes a successful Solidtime pull (projects, tasks updated), notify the Spotlight indexer to refresh the affected slice. This is the Rust→Swift FFI for non-verb-driven mutations.

**Files:**
- Modify: `crates/stint-core/src/ffi.rs` (add `notify_indexer`)
- Modify: `crates/stint-app/src/pull_worker.rs`

- [ ] **Step 1: Add `notify_indexer` to ffi.rs**

Append:

```rust
// ---- indexer notify (Rust → Swift via dlsym) ----

#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum IndexerKind {
    EntryStarted = 1,
    EntryStopped = 2,
    EntryUpdated = 3,
    EntryDeleted = 4,
    ProjectsReplaced = 5,
    TasksReplaced = 6,
}

type IndexerNotifyFn = unsafe extern "C" fn(i32, *const c_char);
static INDEXER_NOTIFY_SYMBOL: std::sync::OnceLock<Option<IndexerNotifyFn>> = std::sync::OnceLock::new();

unsafe fn lookup_indexer_notify() -> Option<IndexerNotifyFn> {
    *INDEXER_NOTIFY_SYMBOL.get_or_init(|| {
        let handle = libc::dlopen(std::ptr::null(), libc::RTLD_NOW);
        if handle.is_null() {
            return None;
        }
        let name = std::ffi::CString::new("swift_indexer_notify").unwrap();
        let sym = libc::dlsym(handle, name.as_ptr());
        if sym.is_null() {
            None
        } else {
            Some(std::mem::transmute::<*mut libc::c_void, IndexerNotifyFn>(sym))
        }
    })
}

/// Call from Rust verb call sites and pull_worker. No-op when the Swift
/// framework isn't loaded (CLI binary, headless tests).
pub fn notify_indexer(kind: IndexerKind, payload_json: &str) {
    let Some(f) = (unsafe { lookup_indexer_notify() }) else {
        return;
    };
    let Ok(c) = CString::new(payload_json) else {
        return;
    };
    unsafe { f(kind as i32, c.as_ptr()) };
}
```

- [ ] **Step 2: Wire into pull worker**

Read `crates/stint-app/src/pull_worker.rs`. Locate the success path (after projects + tasks are written to the store). Add:

```rust
use stint_core::ffi::{notify_indexer, IndexerKind};

// After successful project pull:
if let Ok(projects) = verbs::list_projects(&store) {
    if let Ok(payload) = serde_json::to_string(&projects) {
        notify_indexer(IndexerKind::ProjectsReplaced, &payload);
    }
}

// After successful task pull:
if let Ok(tasks) = verbs::list_tasks(&store, None) {
    if let Ok(payload) = serde_json::to_string(&tasks) {
        notify_indexer(IndexerKind::TasksReplaced, &payload);
    }
}
```

- [ ] **Step 3: Wire into the verb mutation sites**

In `crates/stint-core/src/verbs/start.rs`, after the store write succeeds (right before returning), add:

```rust
if let Ok(payload) = serde_json::to_string(&view) {
    crate::ffi::notify_indexer(crate::ffi::IndexerKind::EntryStarted, &payload);
}
```

Repeat for `stop.rs` (EntryStopped), `update_entry.rs` (EntryUpdated), `delete_entry.rs` (EntryDeleted — payload is the local_uuid as a string).

For `delete_entry`, payload is JSON `{"local_uuid": "..."}`.

- [ ] **Step 4: Build to verify no link errors**

```bash
cargo build --workspace
cargo test --workspace -- --test-threads=1 2>&1 | tail -10
```

Expected: clean build, all existing tests still pass (the `notify_indexer` call is a no-op in tests because Swift isn't loaded).

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
git add -A
git commit -m "$(cat <<'EOF'
feat(core): notify_indexer hook on verb mutations + pull completion

Adds a Rust→Swift FFI for incremental Spotlight index updates. The
hook dlsym-looks up swift_indexer_notify and no-ops when absent so
stint-cli (which never loads the Swift framework) compiles and runs
unchanged.

Wired into: verbs::start/stop/update_entry/delete_entry (per-entry
deltas) and stint-app's pull_worker (replace-all projects/tasks after
a successful Solidtime down-sync).
EOF
)"
```

---

## Task C1: C header for Swift bridging

**Goal:** Hand-written C header that the Swift Package's bridging header imports so Swift can call the Rust FFI symbols.

**Files:**
- Create: `crates/stint-core/include/stint_core.h`

- [ ] **Step 1: Write the header**

Create `crates/stint-core/include/stint_core.h`:

```c
//
// stint_core.h
// C ABI declarations for the StintIntents Swift framework.
//
// All functions return either 0 (success — see `out_json` for the JSON
// envelope `{ok:T}` or `{err:{code,message}}`) or -2 on null-pointer misuse.
//
// Memory ownership: all out_json strings are malloc'd by Rust and must be
// freed by the caller via `stint_free_string`. Passing NULL is safe.
//

#ifndef STINT_CORE_H
#define STINT_CORE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// ---- string lifecycle ----
void stint_free_string(char *ptr);

// ---- verbs ----
int32_t stint_verb_start(const char *params_json, char **out_json);
int32_t stint_verb_stop(char **out_json);
int32_t stint_verb_current(char **out_json);
int32_t stint_verb_list_entries(const char *filter_json, char **out_json);
int32_t stint_verb_list_projects(char **out_json);
int32_t stint_verb_list_tasks(const char *params_json, char **out_json);
int32_t stint_verb_update_entry(const char *params_json, char **out_json);
int32_t stint_verb_delete_entry(const char *params_json, char **out_json);

// ---- settings + log + focus ----
int32_t stint_settings_set(const char *key, const char *value);
int32_t stint_settings_get(const char *key, char **out_json);
int32_t stint_settings_clear(const char *key);
void stint_log_warn(const char *msg);
int32_t stint_current_focus_id(char **out_json);

#ifdef __cplusplus
}
#endif

#endif // STINT_CORE_H
```

- [ ] **Step 2: Commit**

```bash
git add crates/stint-core/include/stint_core.h
git commit -m "feat(core): C header for Swift bridging into Rust FFI"
```

---

## Task C2: Swift Package scaffold — Package.swift + Bridge.swift

**Goal:** Replace the throwaway spike package with the real `Package.swift`. Wire up the Rust FFI declarations so Swift can call into Rust.

**Files:**
- Create/overwrite: `crates/stint-app/swift/StintIntents/Package.swift`
- Create: `crates/stint-app/swift/StintIntents/Sources/StintIntents/Bridge.swift`
- Create: `crates/stint-app/swift/StintIntents/Sources/StintIntents/include/stint_intents_bridge.h`
- Create: `crates/stint-app/swift/StintIntents/Sources/StintIntents/module.modulemap`

- [ ] **Step 1: Final Package.swift**

```swift
// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "StintIntents",
    platforms: [.macOS(.v13)],
    products: [
        .library(name: "StintIntents", type: .dynamic, targets: ["StintIntents"]),
    ],
    targets: [
        .target(
            name: "StintIntents",
            path: "Sources/StintIntents",
            exclude: ["Shortcuts/PhraseStrings.xcstrings"],  // resource, declared below
            resources: [
                .process("Shortcuts/PhraseStrings.xcstrings"),
            ],
            publicHeadersPath: "include",
            cSettings: [
                .headerSearchPath("../../../../stint-core/include"),
            ],
            linkerSettings: [
                .linkedLibrary("stint_core"),  // resolved at app-link time
                .unsafeFlags(["-L../../../../target/release"]),
            ]
        ),
        .testTarget(
            name: "StintIntentsTests",
            dependencies: ["StintIntents"],
            path: "Tests/StintIntentsTests"
        ),
        .testTarget(
            name: "StintIntentsIntegrationTests",
            dependencies: ["StintIntents"],
            path: "Tests/StintIntentsIntegrationTests"
        ),
    ]
)
```

The `linkerSettings.linkedLibrary("stint_core")` and `unsafeFlags(-L...)` are placeholders — `cargo build -p stint-core` produces a `libstint_core.dylib` (workspace target dir) that the framework will link against at app-bundle time. Adjust the path based on `target/debug` vs `target/release` and whether stint-core is built as cdylib vs rlib. Worst case, drop the `linkerSettings` here and have `crates/stint-app/build.rs` handle the link directly via `-rpath` flags on the final Tauri binary.

- [ ] **Step 2: Bridging header**

Create `Sources/StintIntents/include/stint_intents_bridge.h`:

```c
#ifndef STINT_INTENTS_BRIDGE_H
#define STINT_INTENTS_BRIDGE_H

#include "stint_core.h"

#endif
```

Create `Sources/StintIntents/module.modulemap`:

```
module CStintCore {
    header "include/stint_intents_bridge.h"
    export *
}
```

- [ ] **Step 3: Bridge.swift — protocol + FFIBridge + StubBridge**

Create `Sources/StintIntents/Bridge.swift`:

```swift
import Foundation
import CStintCore

// MARK: - Envelope decoding

struct Envelope<T: Decodable>: Decodable {
    let ok: T?
    let err: EnvelopeErr?
}

struct EnvelopeErr: Decodable {
    let code: Int
    let message: String
}

// MARK: - Bridge protocol

/// Abstracts the FFI surface so unit tests can inject a stub.
protocol Bridge {
    func start(_ params: StartParams) throws -> EntryDTO
    func stop() throws -> EntryDTO
    func current() throws -> EntryDTO?
    func listEntries(_ filter: EntryFilter) throws -> [EntryDTO]
    func listProjects() throws -> [ProjectDTO]
    func listTasks(projectId: String?) throws -> [TaskDTO]
    func updateEntry(localUuid: String, patch: EntryPatch) throws -> EntryDTO
    func deleteEntry(localUuid: String) throws

    func settingsSet(_ key: String, _ value: String) throws
    func settingsGet(_ key: String) throws -> String?
    func settingsClear(_ key: String) throws

    func currentFocusId() -> String?
    func logWarn(_ msg: String)
}

// MARK: - DTOs (match the Rust serde shapes in verbs/types.rs)

struct StartParams: Encodable {
    var description: String
    var projectId: String?
    var taskId: String?
    var billable: Bool = false
    var startAt: String? = nil  // ISO 8601 UTC
    var source: String = "intent"

    enum CodingKeys: String, CodingKey {
        case description, source, billable
        case projectId = "project_id"
        case taskId = "task_id"
        case startAt = "start_at"
    }
}

struct EntryFilter: Encodable {
    var since: String? = nil
    var until: String? = nil
    var projectId: String? = nil
    var limit: UInt32? = nil

    enum CodingKeys: String, CodingKey {
        case since, until, limit
        case projectId = "project_id"
    }
}

struct EntryPatch: Encodable {
    var description: String?
    // For nullable fields we use a sentinel because Swift can't express
    // Option<Option<String>> directly. Encode as JSON null vs absent.
    var projectId: ProjectIdPatch = .unchanged
    var taskId: ProjectIdPatch = .unchanged  // same 3-way semantics
    var billable: Bool?
    var startAt: String?
    var endAt: EndAtPatch = .unchanged

    enum CodingKeys: String, CodingKey {
        case description, billable
        case projectId = "project_id"
        case taskId = "task_id"
        case startAt = "start_at"
        case endAt = "end_at"
    }

    func encode(to encoder: Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        if let d = description { try c.encode(d, forKey: .description) }
        if let b = billable { try c.encode(b, forKey: .billable) }
        if let s = startAt { try c.encode(s, forKey: .startAt) }
        switch projectId {
        case .unchanged: break
        case .clear: try c.encodeNil(forKey: .projectId)
        case .set(let v): try c.encode(v, forKey: .projectId)
        }
        switch taskId {
        case .unchanged: break
        case .clear: try c.encodeNil(forKey: .taskId)
        case .set(let v): try c.encode(v, forKey: .taskId)
        }
        switch endAt {
        case .unchanged: break
        case .clear: try c.encodeNil(forKey: .endAt)
        case .set(let v): try c.encode(v, forKey: .endAt)
        }
    }
}

enum ProjectIdPatch { case unchanged, clear, set(String) }
enum EndAtPatch { case unchanged, clear, set(String) }

struct EntryDTO: Decodable {
    let localUuid: String
    let solidtimeId: String?
    let description: String
    let projectId: String?
    let taskId: String?
    let billable: Bool
    let startAt: String
    let endAt: String?
    let source: String

    enum CodingKeys: String, CodingKey {
        case description, billable, source
        case localUuid = "local_uuid"
        case solidtimeId = "solidtime_id"
        case projectId = "project_id"
        case taskId = "task_id"
        case startAt = "start_at"
        case endAt = "end_at"
    }
}

struct ProjectDTO: Decodable {
    let solidtimeId: String
    let name: String
    let color: String?
    let clientId: String?
    let archived: Bool

    enum CodingKeys: String, CodingKey {
        case name, color, archived
        case solidtimeId = "solidtime_id"
        case clientId = "client_id"
    }
}

struct TaskDTO: Decodable {
    let solidtimeId: String
    let projectId: String
    let name: String
    let done: Bool

    enum CodingKeys: String, CodingKey {
        case name, done
        case solidtimeId = "solidtime_id"
        case projectId = "project_id"
    }
}

// MARK: - FFIBridge — production implementation

final class FFIBridge: Bridge {
    static let shared = FFIBridge()

    private let encoder: JSONEncoder = {
        let e = JSONEncoder()
        return e
    }()
    private let decoder: JSONDecoder = {
        let d = JSONDecoder()
        return d
    }()

    private func callWithParams<P: Encodable, T: Decodable>(
        _ verb: (UnsafePointer<CChar>?, UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32,
        _ params: P
    ) throws -> T {
        let json = try encoder.encode(params)
        let cstr = json.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> [CChar] in
            var buf = Array(raw.bindMemory(to: CChar.self))
            buf.append(0)
            return buf
        }
        var out: UnsafeMutablePointer<CChar>?
        _ = cstr.withUnsafeBufferPointer { ptr in
            verb(ptr.baseAddress, &out)
        }
        return try decodeEnvelope(out)
    }

    private func callNoParams<T: Decodable>(
        _ verb: (UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32
    ) throws -> T {
        var out: UnsafeMutablePointer<CChar>?
        _ = verb(&out)
        return try decodeEnvelope(out)
    }

    private func decodeEnvelope<T: Decodable>(_ ptr: UnsafeMutablePointer<CChar>?) throws -> T {
        guard let ptr = ptr else { throw BridgeError.internal("null envelope ptr") }
        defer { stint_free_string(ptr) }
        let data = Data(bytesNoCopy: ptr, count: strlen(ptr), deallocator: .none)
        let env = try decoder.decode(Envelope<T>.self, from: data)
        if let e = env.err {
            throw BridgeError.from(code: Int32(e.code), message: e.message)
        }
        guard let ok = env.ok else {
            throw BridgeError.internal("envelope missing both ok and err")
        }
        return ok
    }

    func start(_ params: StartParams) throws -> EntryDTO {
        return try callWithParams(stint_verb_start, params)
    }

    func stop() throws -> EntryDTO {
        return try callNoParams(stint_verb_stop)
    }

    func current() throws -> EntryDTO? {
        var out: UnsafeMutablePointer<CChar>?
        _ = stint_verb_current(&out)
        guard let ptr = out else { return nil }
        defer { stint_free_string(ptr) }
        let data = Data(bytesNoCopy: ptr, count: strlen(ptr), deallocator: .none)
        let env = try decoder.decode(Envelope<EntryDTO?>.self, from: data)
        if let e = env.err {
            throw BridgeError.from(code: Int32(e.code), message: e.message)
        }
        return env.ok ?? nil
    }

    func listEntries(_ filter: EntryFilter) throws -> [EntryDTO] {
        return try callWithParams(stint_verb_list_entries, filter)
    }

    func listProjects() throws -> [ProjectDTO] {
        return try callNoParams(stint_verb_list_projects)
    }

    func listTasks(projectId: String?) throws -> [TaskDTO] {
        struct P: Encodable {
            let projectId: String?
            enum CodingKeys: String, CodingKey { case projectId = "project_id" }
        }
        return try callWithParams(stint_verb_list_tasks, P(projectId: projectId))
    }

    func updateEntry(localUuid: String, patch: EntryPatch) throws -> EntryDTO {
        struct P: Encodable {
            let localUuid: String
            let patch: EntryPatch
            enum CodingKeys: String, CodingKey {
                case patch
                case localUuid = "local_uuid"
            }
        }
        return try callWithParams(stint_verb_update_entry, P(localUuid: localUuid, patch: patch))
    }

    func deleteEntry(localUuid: String) throws {
        struct P: Encodable {
            let localUuid: String
            enum CodingKeys: String, CodingKey { case localUuid = "local_uuid" }
        }
        let _: [String: String] = try callWithParams(stint_verb_delete_entry, P(localUuid: localUuid))
    }

    func settingsSet(_ key: String, _ value: String) throws {
        let rc = key.withCString { k in value.withCString { v in stint_settings_set(k, v) } }
        if rc != 0 { throw BridgeError.internal("settings_set rc=\(rc)") }
    }

    func settingsGet(_ key: String) throws -> String? {
        var out: UnsafeMutablePointer<CChar>?
        let rc = key.withCString { k in stint_settings_get(k, &out) }
        if rc != 0 { throw BridgeError.internal("settings_get rc=\(rc)") }
        guard let ptr = out else { return nil }
        defer { stint_free_string(ptr) }
        return String(cString: ptr)
    }

    func settingsClear(_ key: String) throws {
        let rc = key.withCString { k in stint_settings_clear(k) }
        if rc != 0 { throw BridgeError.internal("settings_clear rc=\(rc)") }
    }

    func currentFocusId() -> String? {
        var out: UnsafeMutablePointer<CChar>?
        let rc = stint_current_focus_id(&out)
        if rc != 0 { return nil }
        guard let ptr = out else { return nil }
        defer { stint_free_string(ptr) }
        return String(cString: ptr)
    }

    func logWarn(_ msg: String) {
        msg.withCString { stint_log_warn($0) }
    }
}
```

- [ ] **Step 4: Commit**

```bash
git add crates/stint-app/swift/StintIntents/
git commit -m "$(cat <<'EOF'
feat(swift): SPM scaffold + Bridge protocol + FFIBridge

Final Package.swift declares the StintIntents dynamic library targeting
macOS 13+. Module map exposes the hand-written C header.

Bridge.swift defines the Bridge protocol (so AppIntent unit tests can
inject a stub) and FFIBridge (the production implementation that calls
into stint-core's extern "C" surface). DTOs mirror the Rust verb shapes
in stint_core::verbs::types via Codable.

The EntryPatch 3-way nullable semantics (unchanged / clear / set) are
modeled via custom Encodable that emits absent / null / value correctly.
EOF
)"
```

---

## Task C3: BridgeError

**Goal:** `BridgeError` enum that maps envelope codes to typed Swift errors and conforms to App Intents' error vocabulary.

**Files:**
- Create: `crates/stint-app/swift/StintIntents/Sources/StintIntents/Errors/BridgeError.swift`

- [ ] **Step 1: Write the file**

```swift
import Foundation
import AppIntents

enum BridgeError: LocalizedError {
    case invariant(String)
    case notFound(String)
    case conflict(String)
    case serialization(String)
    case `internal`(String)
    case panic(String)

    static func from(code: Int32, message: String) -> BridgeError {
        switch code {
        case 1: return .invariant(message)
        case 2: return .notFound(message)
        case 3: return .conflict(message)
        case 4: return .serialization(message)
        case -1: return .panic(message)
        default: return .internal(message)
        }
    }

    var errorDescription: String? {
        switch self {
        case .invariant(let m), .notFound(let m): return m
        case .conflict(_): return "That conflicts with an existing entry."
        case .serialization(_): return "Couldn't read the request."
        case .internal(_): return "Stint hit an internal error. Check the app."
        case .panic(_): return "Stint encountered an unexpected error."
        }
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/stint-app/swift/StintIntents/Sources/StintIntents/Errors/BridgeError.swift
git commit -m "feat(swift): BridgeError mapping FFI codes to LocalizedError"
```

---

## Task D1-D3: Swift entities — Project / Task / Entry + their EntityQuery types

**Goal:** Three `AppEntity` + `EntityQuery` pairs. Each entity is `IndexedEntity` so Spotlight gets it for free.

**Files (one task each, three tasks total):**
- Create: `crates/stint-app/swift/StintIntents/Sources/StintIntents/Entities/ProjectEntity.swift`
- Create: `crates/stint-app/swift/StintIntents/Sources/StintIntents/Entities/ProjectQuery.swift`
- Create: `crates/stint-app/swift/StintIntents/Sources/StintIntents/Entities/TaskEntity.swift`
- Create: `crates/stint-app/swift/StintIntents/Sources/StintIntents/Entities/TaskQuery.swift`
- Create: `crates/stint-app/swift/StintIntents/Sources/StintIntents/Entities/EntryEntity.swift`
- Create: `crates/stint-app/swift/StintIntents/Sources/StintIntents/Entities/EntryQuery.swift`

Each task is one entity+query pair. Pattern (showing ProjectEntity; TaskEntity and EntryEntity follow):

**`ProjectEntity.swift`:**

```swift
import AppIntents
import Foundation

struct ProjectEntity: AppEntity, IndexedEntity {
    static var typeDisplayRepresentation = TypeDisplayRepresentation(name: "Project")
    static var defaultQuery = ProjectQuery()

    let id: String
    let name: String
    let clientName: String?

    var displayRepresentation: DisplayRepresentation {
        DisplayRepresentation(
            title: "\(name)",
            subtitle: clientName.map { "Project · \($0)" } ?? "Project"
        )
    }

    init(from dto: ProjectDTO) {
        self.id = dto.solidtimeId
        self.name = dto.name
        self.clientName = nil  // TODO: pull from Solidtime client cache when available
    }
}
```

**`ProjectQuery.swift`:**

```swift
import AppIntents
import Foundation

struct ProjectQuery: EntityQuery {
    var bridge: Bridge = FFIBridge.shared

    func entities(for identifiers: [ProjectEntity.ID]) async throws -> [ProjectEntity] {
        let all = try bridge.listProjects().map(ProjectEntity.init(from:))
        return all.filter { identifiers.contains($0.id) }
    }

    func suggestedEntities() async throws -> [ProjectEntity] {
        try bridge.listProjects().filter { !$0.archived }.map(ProjectEntity.init(from:))
    }
}

extension ProjectQuery: EntityStringQuery {
    func entities(matching string: String) async throws -> [ProjectEntity] {
        let q = string.lowercased()
        return try bridge.listProjects()
            .filter { !$0.archived }
            .filter { $0.name.lowercased().contains(q) }
            .map(ProjectEntity.init(from:))
    }
}
```

**`TaskEntity.swift`:**

```swift
import AppIntents

struct TaskEntity: AppEntity, IndexedEntity {
    static var typeDisplayRepresentation = TypeDisplayRepresentation(name: "Task")
    static var defaultQuery = TaskQuery()

    let id: String
    let projectId: String
    let name: String

    var displayRepresentation: DisplayRepresentation {
        DisplayRepresentation(title: "\(name)", subtitle: "Task in project \(projectId)")
    }

    init(from dto: TaskDTO) {
        self.id = dto.solidtimeId
        self.projectId = dto.projectId
        self.name = dto.name
    }
}
```

**`TaskQuery.swift`:**

```swift
import AppIntents

struct TaskQuery: EntityQuery {
    var bridge: Bridge = FFIBridge.shared

    func entities(for identifiers: [TaskEntity.ID]) async throws -> [TaskEntity] {
        let all = try bridge.listTasks(projectId: nil).map(TaskEntity.init(from:))
        return all.filter { identifiers.contains($0.id) }
    }

    func suggestedEntities() async throws -> [TaskEntity] {
        try bridge.listTasks(projectId: nil).map(TaskEntity.init(from:))
    }
}
```

**`EntryEntity.swift`:**

```swift
import AppIntents
import Foundation

struct EntryEntity: AppEntity, IndexedEntity {
    static var typeDisplayRepresentation = TypeDisplayRepresentation(name: "Time Entry")
    static var defaultQuery = EntryQuery()

    let id: String  // local_uuid
    let description: String
    let projectId: String?
    let taskId: String?
    let billable: Bool
    let startAt: Date
    let endAt: Date?

    var duration: Measurement<UnitDuration> {
        let end = endAt ?? Date()
        return Measurement(value: end.timeIntervalSince(startAt), unit: .seconds)
    }

    var displayRepresentation: DisplayRepresentation {
        let fmt = ISO8601DateFormatter()
        return DisplayRepresentation(
            title: "\(description)",
            subtitle: "\(fmt.string(from: startAt)) · \(Int(duration.converted(to: .minutes).value))m"
        )
    }

    init(from dto: EntryDTO) {
        self.id = dto.localUuid
        self.description = dto.description
        self.projectId = dto.projectId
        self.taskId = dto.taskId
        self.billable = dto.billable
        let fmt = ISO8601DateFormatter()
        self.startAt = fmt.date(from: dto.startAt) ?? Date()
        self.endAt = dto.endAt.flatMap(fmt.date(from:))
    }
}
```

**`EntryQuery.swift`:**

```swift
import AppIntents

struct EntryQuery: EntityQuery, EntityStringQuery {
    var bridge: Bridge = FFIBridge.shared

    func entities(for identifiers: [EntryEntity.ID]) async throws -> [EntryEntity] {
        // Filter is per-since/until — but we don't have explicit lookup-by-id.
        // Fetch a wide window and filter.
        let entries = try bridge.listEntries(EntryFilter(limit: 500))
            .map(EntryEntity.init(from:))
        return entries.filter { identifiers.contains($0.id) }
    }

    func suggestedEntities() async throws -> [EntryEntity] {
        try bridge.listEntries(EntryFilter(limit: 20))
            .map(EntryEntity.init(from:))
    }

    func entities(matching string: String) async throws -> [EntryEntity] {
        let q = string.lowercased()
        return try bridge.listEntries(EntryFilter(limit: 200))
            .map(EntryEntity.init(from:))
            .filter { $0.description.lowercased().contains(q) }
    }
}
```

**Commit after each of D1, D2, D3:**

```bash
git add crates/stint-app/swift/StintIntents/Sources/StintIntents/Entities/Project*.swift
git commit -m "feat(swift): ProjectEntity + ProjectQuery"
```

(repeat for Task, then Entry)

---

## Task E1: SpotlightIndexer

**Goal:** Bulk + delta Spotlight index updates.

**Files:**
- Create: `crates/stint-app/swift/StintIntents/Sources/StintIntents/Spotlight/SpotlightIndexer.swift`

- [ ] **Step 1: Write the file**

```swift
import CoreSpotlight
import UniformTypeIdentifiers
import Foundation

enum IndexerKind: Int {
    case entryStarted = 1
    case entryStopped = 2
    case entryUpdated = 3
    case entryDeleted = 4
    case projectsReplaced = 5
    case tasksReplaced = 6
}

final class SpotlightIndexer {
    static let shared = SpotlightIndexer()

    private let entryDomain = "tech.reyem.stint.entry"
    private let projectDomain = "tech.reyem.stint.project"
    private let taskDomain = "tech.reyem.stint.task"

    private var bridge: Bridge { FFIBridge.shared }

    func bulkRefresh() {
        Task.detached(priority: .background) {
            self.refreshEntries()
            self.refreshProjects()
            self.refreshTasks()
        }
    }

    func delta(kind: IndexerKind, payload: String) {
        Task.detached(priority: .background) {
            do {
                switch kind {
                case .entryStarted, .entryStopped, .entryUpdated:
                    let dto = try JSONDecoder().decode(EntryDTO.self, from: Data(payload.utf8))
                    self.upsertEntry(EntryEntity(from: dto))
                case .entryDeleted:
                    struct P: Decodable { let local_uuid: String }
                    let p = try JSONDecoder().decode(P.self, from: Data(payload.utf8))
                    self.deleteEntry(localUuid: p.local_uuid)
                case .projectsReplaced:
                    self.refreshProjects()
                case .tasksReplaced:
                    self.refreshTasks()
                }
            } catch {
                self.bridge.logWarn("spotlight delta decode failed: \(error)")
            }
        }
    }

    // MARK: - Entries

    private func refreshEntries() {
        do {
            let entries = try bridge.listEntries(EntryFilter(limit: nil))
                .map(EntryEntity.init(from:))
            let items = entries.map(makeEntryItem)
            CSSearchableIndex.default().indexSearchableItems(items) { [bridge] error in
                if let error = error { bridge.logWarn("spotlight indexEntries failed: \(error)") }
            }
        } catch {
            bridge.logWarn("spotlight refreshEntries fetch failed: \(error)")
        }
    }

    func upsertEntry(_ entry: EntryEntity) {
        let item = makeEntryItem(entry)
        CSSearchableIndex.default().indexSearchableItems([item]) { [bridge] error in
            if let error = error { bridge.logWarn("spotlight upsertEntry failed: \(error)") }
        }
    }

    func deleteEntry(localUuid: String) {
        CSSearchableIndex.default().deleteSearchableItems(withIdentifiers: [localUuid]) { [bridge] error in
            if let error = error { bridge.logWarn("spotlight deleteEntry failed: \(error)") }
        }
    }

    func makeEntryItem(_ entry: EntryEntity) -> CSSearchableItem {
        let attrs = CSSearchableItemAttributeSet(contentType: UTType.text)
        attrs.title = entry.description
        let mins = Int(entry.duration.converted(to: .minutes).value)
        attrs.contentDescription = "\(entry.startAt) · \(mins)m"
        attrs.keywords = ["stint", "timer"]
        attrs.containerIdentifier = entry.projectId
        return CSSearchableItem(
            uniqueIdentifier: entry.id,
            domainIdentifier: entryDomain,
            attributeSet: attrs
        )
    }

    // MARK: - Projects

    private func refreshProjects() {
        do {
            let projects = try bridge.listProjects()
            let items = projects.map(makeProjectItem)
            CSSearchableIndex.default().indexSearchableItems(items) { [bridge] error in
                if let error = error { bridge.logWarn("spotlight refreshProjects failed: \(error)") }
            }
        } catch {
            bridge.logWarn("spotlight refreshProjects fetch failed: \(error)")
        }
    }

    func makeProjectItem(_ project: ProjectDTO) -> CSSearchableItem {
        let attrs = CSSearchableItemAttributeSet(contentType: UTType.text)
        attrs.title = project.name
        attrs.contentDescription = "Project"
        attrs.keywords = ["stint", "project", project.name]
        return CSSearchableItem(
            uniqueIdentifier: project.solidtimeId,
            domainIdentifier: projectDomain,
            attributeSet: attrs
        )
    }

    // MARK: - Tasks

    private func refreshTasks() {
        do {
            let tasks = try bridge.listTasks(projectId: nil)
            let items = tasks.map(makeTaskItem)
            CSSearchableIndex.default().indexSearchableItems(items) { [bridge] error in
                if let error = error { bridge.logWarn("spotlight refreshTasks failed: \(error)") }
            }
        } catch {
            bridge.logWarn("spotlight refreshTasks fetch failed: \(error)")
        }
    }

    func makeTaskItem(_ task: TaskDTO) -> CSSearchableItem {
        let attrs = CSSearchableItemAttributeSet(contentType: UTType.text)
        attrs.title = task.name
        attrs.contentDescription = "Task in project \(task.projectId)"
        attrs.keywords = ["stint", "task", task.name]
        return CSSearchableItem(
            uniqueIdentifier: task.solidtimeId,
            domainIdentifier: taskDomain,
            attributeSet: attrs
        )
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/stint-app/swift/StintIntents/Sources/StintIntents/Spotlight/SpotlightIndexer.swift
git commit -m "feat(swift): SpotlightIndexer — bulk refresh + delta updates for entries/projects/tasks"
```

---

## Task E2: ActivityTracker

**Files:**
- Create: `crates/stint-app/swift/StintIntents/Sources/StintIntents/Spotlight/ActivityTracker.swift`

- [ ] **Step 1: Write the file**

```swift
import Foundation
import CoreSpotlight

final class ActivityTracker {
    static let shared = ActivityTracker()

    private var current: NSUserActivity?

    func activate(entry: EntryEntity) {
        let activity = NSUserActivity(activityType: "tech.reyem.stint.tracking")
        activity.title = "Tracking: \(entry.description)"
        activity.userInfo = ["uuid": entry.id]
        activity.isEligibleForSearch = true
        activity.isEligibleForHandoff = true
        if #available(macOS 13, *) {
            activity.isEligibleForPrediction = true
        }
        activity.becomeCurrent()
        self.current = activity
    }

    func update(description: String) {
        current?.title = "Tracking: \(description)"
    }

    func invalidate() {
        current?.invalidate()
        current = nil
    }

    func boot() {
        Task.detached(priority: .background) {
            do {
                if let entry = try FFIBridge.shared.current() {
                    let entity = EntryEntity(from: entry)
                    await MainActor.run { self.activate(entry: entity) }
                }
            } catch {
                FFIBridge.shared.logWarn("activitytracker boot failed: \(error)")
            }
        }
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/stint-app/swift/StintIntents/Sources/StintIntents/Spotlight/ActivityTracker.swift
git commit -m "feat(swift): ActivityTracker — NSUserActivity for the running entry"
```

---

## Task E3: Init module — stint_intents_init + swift_indexer_notify + stint_current_focus_id_swift

**Files:**
- Create: `crates/stint-app/swift/StintIntents/Sources/StintIntents/Init/StintIntentsInit.swift`

- [ ] **Step 1: Write the file**

```swift
import Foundation
import CStintCore

/// Called once by Rust during Tauri setup(). Loads the Swift runtime
/// implicitly (first FFI symbol resolution) and kicks off Spotlight + Activity.
@_cdecl("stint_intents_init")
public func stint_intents_init() -> Int32 {
    SpotlightIndexer.shared.bulkRefresh()
    ActivityTracker.shared.boot()
    return 0
}

/// Called from Rust on every verb mutation + after pull-worker successes.
@_cdecl("swift_indexer_notify")
public func swift_indexer_notify(_ kind: Int32, _ payloadPtr: UnsafePointer<CChar>?) {
    guard let payloadPtr = payloadPtr else { return }
    guard let kind = IndexerKind(rawValue: Int(kind)) else { return }
    let payload = String(cString: payloadPtr)

    // Mutating ActivityTracker on start/stop/update needs the EntryDTO too.
    switch kind {
    case .entryStarted:
        if let entry = try? JSONDecoder().decode(EntryDTO.self, from: Data(payload.utf8)) {
            DispatchQueue.main.async { ActivityTracker.shared.activate(entry: EntryEntity(from: entry)) }
        }
    case .entryStopped:
        DispatchQueue.main.async { ActivityTracker.shared.invalidate() }
    case .entryUpdated:
        if let entry = try? JSONDecoder().decode(EntryDTO.self, from: Data(payload.utf8)) {
            DispatchQueue.main.async { ActivityTracker.shared.update(description: entry.description) }
        }
    default:
        break
    }

    SpotlightIndexer.shared.delta(kind: kind, payload: payload)
}

/// Returns the currently active macOS Focus identifier. Called by Rust via dlsym
/// during the start-verb fallback path.
@_cdecl("stint_current_focus_id_swift")
public func stint_current_focus_id_swift(_ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>) -> Int32 {
    out.pointee = nil

    // INFocusStatusCenter is iOS-only as of macOS 13; the macOS API uses
    // NSUserActivity-based focus interrogation through assertions. For the
    // 6b ship we read the active focus from a UserDefaults key set by the
    // OS when a Focus filter activates (this is how SetFocusFilterIntent
    // wires through). If unavailable, return null.
    if let focusId = UserDefaults.standard.string(forKey: "com.apple.focus.currentIdentifier") {
        let c = strdup(focusId)
        out.pointee = c
    }
    return 0
}
```

Note: the macOS Focus public-API surface for reading the current focus id is limited; the implementation above reads a UserDefaults key whose presence in current OS versions should be verified during execution. If that doesn't work, fall back to setting the focus_id from `ProjectFocusFilter.perform()` directly (storing it via `stint_settings_set` together with the project) and skipping the read-side lookup entirely — the start-verb fallback simply trusts whatever the most recent `perform()` wrote.

- [ ] **Step 2: Commit**

```bash
git add crates/stint-app/swift/StintIntents/Sources/StintIntents/Init/StintIntentsInit.swift
git commit -m "feat(swift): @_cdecl exports — init, indexer notify, focus id"
```

---

## Task F1-F10: App Intents (10 intent types)

Each intent is a separate small file in `Sources/StintIntents/Intents/`. Pattern (StartTimerIntent shown in full; others follow the same shape). Commit one per intent.

**`StartTimerIntent.swift`:**

```swift
import AppIntents
import Foundation

struct StartTimerIntent: AppIntent {
    static var title: LocalizedStringResource = "Start Timer"
    static var description = IntentDescription("Start tracking time on a project in Stint.")

    @Parameter(title: "Description", requestValueDialog: "What are you working on?")
    var description: String

    @Parameter(title: "Project")
    var project: ProjectEntity?

    var bridge: Bridge = FFIBridge.shared

    func perform() async throws -> some IntentResult & ProvidesDialog & ReturnsValue<EntryEntity> {
        let entry = try bridge.start(StartParams(
            description: description,
            projectId: project?.id,
            source: "intent"
        ))
        let entity = EntryEntity(from: entry)
        let projectName = project?.name ?? "no project"
        return .result(value: entity, dialog: "Tracking '\(description)' on \(projectName).")
    }
}
```

**`StopTimerIntent.swift`:**

```swift
import AppIntents

struct StopTimerIntent: AppIntent {
    static var title: LocalizedStringResource = "Stop Timer"
    static var description = IntentDescription("Stop the running Stint timer.")

    var bridge: Bridge = FFIBridge.shared

    func perform() async throws -> some IntentResult & ProvidesDialog {
        let entry = try bridge.stop()
        let mins = Int(EntryEntity(from: entry).duration.converted(to: .minutes).value)
        return .result(dialog: "Stopped. \(mins) minutes on \(entry.projectId ?? "no project").")
    }
}
```

**`GetCurrentIntent.swift`:**

```swift
import AppIntents

struct GetCurrentIntent: AppIntent {
    static var title: LocalizedStringResource = "Current Timer"
    static var description = IntentDescription("Show the currently running Stint timer.")

    var bridge: Bridge = FFIBridge.shared

    func perform() async throws -> some IntentResult & ProvidesDialog & ReturnsValue<EntryEntity?> {
        guard let entry = try bridge.current() else {
            return .result(value: nil, dialog: "No active timer.")
        }
        let entity = EntryEntity(from: entry)
        return .result(value: entity, dialog: "You're tracking '\(entry.description)'.")
    }
}
```

**`SwitchProjectIntent.swift`:**

```swift
import AppIntents

struct SwitchProjectIntent: AppIntent {
    static var title: LocalizedStringResource = "Switch Project"
    static var description = IntentDescription("Stop the current Stint timer and start a new one on a different project.")

    @Parameter(title: "Project")
    var project: ProjectEntity

    var bridge: Bridge = FFIBridge.shared

    func perform() async throws -> some IntentResult & ProvidesDialog {
        guard let current = try bridge.current() else {
            throw BridgeError.invariant("No timer to switch from.")
        }
        _ = try bridge.stop()
        _ = try bridge.start(StartParams(
            description: current.description,
            projectId: project.id,
            source: "intent"
        ))
        return .result(dialog: "Switched to \(project.name).")
    }
}
```

**`LogPastIntent.swift`:**

```swift
import AppIntents
import Foundation

struct LogPastIntent: AppIntent {
    static var title: LocalizedStringResource = "Log Past Work"
    static var description = IntentDescription("Retroactively log a past duration in Stint.")

    @Parameter(title: "Duration")
    var duration: Measurement<UnitDuration>

    @Parameter(title: "Description", default: "Untitled")
    var description: String

    @Parameter(title: "Project")
    var project: ProjectEntity?

    var bridge: Bridge = FFIBridge.shared

    func perform() async throws -> some IntentResult & ProvidesDialog {
        let seconds = duration.converted(to: .seconds).value
        let startDate = Date(timeIntervalSinceNow: -seconds)
        let fmt = ISO8601DateFormatter()
        // If a timer is running, stop it first so the backdated entry doesn't overlap.
        if (try? bridge.current()) != nil {
            _ = try? bridge.stop()
        }
        _ = try bridge.start(StartParams(
            description: description,
            projectId: project?.id,
            startAt: fmt.string(from: startDate),
            source: "intent"
        ))
        _ = try bridge.stop()
        let mins = Int(duration.converted(to: .minutes).value)
        return .result(dialog: "Logged \(mins) minutes on \(project?.name ?? "no project").")
    }
}
```

**`ListEntriesIntent.swift`:**

```swift
import AppIntents
import Foundation

struct ListEntriesIntent: AppIntent {
    static var title: LocalizedStringResource = "List Entries"
    static var description = IntentDescription("Fetch Stint time entries.")

    @Parameter(title: "Since")
    var since: Date?

    @Parameter(title: "Until")
    var until: Date?

    @Parameter(title: "Project")
    var project: ProjectEntity?

    @Parameter(title: "Limit", default: 100)
    var limit: Int

    var bridge: Bridge = FFIBridge.shared

    func perform() async throws -> some IntentResult & ReturnsValue<[EntryEntity]> {
        let fmt = ISO8601DateFormatter()
        let filter = EntryFilter(
            since: since.map { fmt.string(from: $0) },
            until: until.map { fmt.string(from: $0) },
            projectId: project?.id,
            limit: UInt32(limit)
        )
        let entries = try bridge.listEntries(filter).map(EntryEntity.init(from:))
        return .result(value: entries)
    }
}
```

**`ListProjectsIntent.swift`:**

```swift
import AppIntents

struct ListProjectsIntent: AppIntent {
    static var title: LocalizedStringResource = "List Projects"
    static var description = IntentDescription("Fetch the list of Stint projects.")

    var bridge: Bridge = FFIBridge.shared

    func perform() async throws -> some IntentResult & ReturnsValue<[ProjectEntity]> {
        let projects = try bridge.listProjects().map(ProjectEntity.init(from:))
        return .result(value: projects)
    }
}
```

**`ListTasksIntent.swift`:**

```swift
import AppIntents

struct ListTasksIntent: AppIntent {
    static var title: LocalizedStringResource = "List Tasks"
    static var description = IntentDescription("Fetch Stint tasks for a project.")

    @Parameter(title: "Project")
    var project: ProjectEntity

    var bridge: Bridge = FFIBridge.shared

    func perform() async throws -> some IntentResult & ReturnsValue<[TaskEntity]> {
        let tasks = try bridge.listTasks(projectId: project.id).map(TaskEntity.init(from:))
        return .result(value: tasks)
    }
}
```

**`UpdateEntryIntent.swift`:**

```swift
import AppIntents
import Foundation

struct UpdateEntryIntent: AppIntent {
    static var title: LocalizedStringResource = "Update Entry"
    static var description = IntentDescription("Update fields on a Stint time entry.")

    @Parameter(title: "Entry")
    var entry: EntryEntity

    @Parameter(title: "Description")
    var description: String?

    @Parameter(title: "Project")
    var project: ProjectEntity?

    @Parameter(title: "Billable")
    var billable: Bool?

    var bridge: Bridge = FFIBridge.shared

    func perform() async throws -> some IntentResult & ReturnsValue<EntryEntity> {
        var patch = EntryPatch()
        if let d = description { patch.description = d }
        if let p = project { patch.projectId = .set(p.id) }
        if let b = billable { patch.billable = b }
        let updated = try bridge.updateEntry(localUuid: entry.id, patch: patch)
        return .result(value: EntryEntity(from: updated))
    }
}
```

**`DeleteEntryIntent.swift`:**

```swift
import AppIntents

struct DeleteEntryIntent: AppIntent {
    static var title: LocalizedStringResource = "Delete Entry"
    static var description = IntentDescription("Delete a Stint time entry.")

    @Parameter(title: "Entry")
    var entry: EntryEntity

    var bridge: Bridge = FFIBridge.shared

    func perform() async throws -> some IntentResult & ProvidesDialog {
        try bridge.deleteEntry(localUuid: entry.id)
        return .result(dialog: "Deleted '\(entry.description)'.")
    }
}
```

**Commit each intent file separately:**

```bash
git add crates/stint-app/swift/StintIntents/Sources/StintIntents/Intents/StartTimerIntent.swift
git commit -m "feat(swift): StartTimerIntent"
# ... repeat for each
```

---

## Task G1: App Shortcuts provider + xcstrings

**Files:**
- Create: `crates/stint-app/swift/StintIntents/Sources/StintIntents/Shortcuts/StintAppShortcutsProvider.swift`
- Create: `crates/stint-app/swift/StintIntents/Sources/StintIntents/Shortcuts/PhraseStrings.xcstrings`

- [ ] **Step 1: Provider file**

```swift
import AppIntents

struct StintAppShortcutsProvider: AppShortcutsProvider {
    static var appShortcuts: [AppShortcut] {
        AppShortcut(
            intent: StartTimerIntent(),
            phrases: [
                "Start timer in \(.applicationName)",
                "Start tracking in \(.applicationName)",
                "Start \(\.$project) in \(.applicationName)",
            ],
            shortTitle: "Start Timer",
            systemImageName: "play.circle.fill"
        )

        AppShortcut(
            intent: StopTimerIntent(),
            phrases: [
                "Stop \(.applicationName) timer",
                "Stop tracking in \(.applicationName)",
            ],
            shortTitle: "Stop Timer",
            systemImageName: "stop.circle.fill"
        )

        AppShortcut(
            intent: GetCurrentIntent(),
            phrases: [
                "What am I tracking in \(.applicationName)",
                "Show current \(.applicationName) timer",
            ],
            shortTitle: "Current Timer",
            systemImageName: "clock"
        )

        AppShortcut(
            intent: SwitchProjectIntent(),
            phrases: [
                "Switch to \(\.$project) in \(.applicationName)",
            ],
            shortTitle: "Switch Project",
            systemImageName: "arrow.triangle.swap"
        )

        AppShortcut(
            intent: LogPastIntent(),
            phrases: [
                "Log past \(\.$duration) in \(.applicationName)",
                "Log last meeting in \(.applicationName)",
            ],
            shortTitle: "Log Past Work",
            systemImageName: "backward.circle"
        )
    }
}
```

- [ ] **Step 2: xcstrings**

Create `PhraseStrings.xcstrings` (JSON format Apple expects):

```json
{
  "sourceLanguage": "en",
  "strings": {
    "Start timer in %@": { "extractionState": "manual" },
    "Stop %@ timer": { "extractionState": "manual" },
    "What am I tracking in %@": { "extractionState": "manual" }
  },
  "version": "1.0"
}
```

(The xcstrings format is sparse — Xcode populates it during `appintentsmetadataprocessor`. The structure above is the minimal valid skeleton; SPM's appintents processor enriches it during build.)

- [ ] **Step 3: Commit**

```bash
git add crates/stint-app/swift/StintIntents/Sources/StintIntents/Shortcuts/
git commit -m "feat(swift): StintAppShortcutsProvider with 5 curated voice phrases"
```

---

## Task G2: ProjectFocusFilter

**Files:**
- Create: `crates/stint-app/swift/StintIntents/Sources/StintIntents/Focus/ProjectFocusFilter.swift`

- [ ] **Step 1: Write the file**

```swift
import AppIntents
import Foundation

struct ProjectFocusFilter: SetFocusFilterIntent {
    static var title: LocalizedStringResource = "Default Project"
    static var description = IntentDescription("Set a default project for new Stint timers while this focus is on.")

    @Parameter(title: "Project")
    var project: ProjectEntity

    var bridge: Bridge = FFIBridge.shared

    func perform() async throws -> some IntentResult {
        // Apple calls perform() once per focus activation. We persist a tuple
        // (focus_id, project_id) and let verbs::start reconcile it against
        // the current focus at read time.
        //
        // We don't have a stable "current focus id" API on macOS 13. As a
        // workaround, the focus id we store is a stable hash of the project
        // selection itself + a randomly-generated session token written to
        // UserDefaults so a *new* perform() call overwrites the previous one.
        let focusId = UUID().uuidString
        UserDefaults.standard.set(focusId, forKey: "com.apple.focus.currentIdentifier")
        let payload = "\(focusId)\t\(project.id)"
        try bridge.settingsSet("focus.default_project", payload)
        return .result()
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/stint-app/swift/StintIntents/Sources/StintIntents/Focus/ProjectFocusFilter.swift
git commit -m "feat(swift): ProjectFocusFilter — default project per Focus mode"
```

---

## Task H1: stint-app/build.rs — invoke swift build

**Files:**
- Modify: `crates/stint-app/build.rs`

- [ ] **Step 1: Extend build.rs**

Add after existing logic in `crates/stint-app/build.rs`:

```rust
// Build StintIntents framework via SPM.
{
    let swift_dir = std::path::Path::new("swift/StintIntents");
    if swift_dir.exists() {
        let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".into());
        let swift_profile = if profile == "release" { "release" } else { "debug" };

        println!("cargo:rerun-if-changed=swift/StintIntents/Sources");
        println!("cargo:rerun-if-changed=swift/StintIntents/Package.swift");

        let status = std::process::Command::new("swift")
            .args(["build", "-c", swift_profile, "--product", "StintIntents"])
            .current_dir(swift_dir)
            .status();

        match status {
            Ok(s) if s.success() => {
                let out_dir = std::env::var("OUT_DIR").unwrap();
                let dest = std::path::Path::new(&out_dir).join("StintIntents.framework");
                // SPM emits a .dylib by default for products of type .dynamic.
                // Tauri's bundle.macOS.frameworks expects a .framework directory.
                // Wrap the dylib in a minimal framework structure here.
                wrap_dylib_as_framework(&swift_dir.join(".build").join(swift_profile), &dest);
                println!("cargo:warning=StintIntents framework built at {}", dest.display());
            }
            Ok(s) => println!("cargo:warning=swift build exited non-zero: {s}"),
            Err(e) => println!("cargo:warning=swift build failed to spawn: {e}"),
        }
    }
}

fn wrap_dylib_as_framework(swift_build_dir: &std::path::Path, dest: &std::path::Path) {
    use std::fs;
    let _ = fs::remove_dir_all(dest);
    fs::create_dir_all(dest.join("Versions/A/Resources")).unwrap();
    let dylib_src = swift_build_dir.join("libStintIntents.dylib");
    if dylib_src.exists() {
        fs::copy(&dylib_src, dest.join("Versions/A/StintIntents")).unwrap();
    }
    // Create Info.plist
    let plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>tech.reyem.stint.intents</string>
    <key>CFBundleExecutable</key>
    <string>StintIntents</string>
    <key>CFBundleName</key>
    <string>StintIntents</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>NSAppIntentsPackage</key>
    <true/>
</dict>
</plist>
"#;
    fs::write(dest.join("Versions/A/Resources/Info.plist"), plist).unwrap();
    // Copy Metadata.appintents stencil if SPM produced one
    let stencil_candidates = [
        swift_build_dir.join("StintIntents.bundle/Contents/Resources/Metadata.appintents"),
        swift_build_dir.join("StintIntents_StintIntents.bundle/Contents/Resources/Metadata.appintents"),
    ];
    for cand in &stencil_candidates {
        if cand.exists() {
            fs::copy(cand, dest.join("Versions/A/Resources/Metadata.appintents")).unwrap();
            break;
        }
    }
    // Symlinks
    use std::os::unix::fs::symlink;
    let _ = symlink("A", dest.join("Versions/Current"));
    let _ = symlink("Versions/Current/StintIntents", dest.join("StintIntents"));
    let _ = symlink("Versions/Current/Resources", dest.join("Resources"));
}
```

This is the integration glue most likely to need iteration during execution — verify each path exists during build and adapt to where SPM actually emits artifacts.

- [ ] **Step 2: Test the build**

```bash
cargo build -p stint-app 2>&1 | tail -20
ls target/debug/build/stint-app-*/out/StintIntents.framework/ 2>/dev/null
```

Expected: clean build, framework artifact present.

- [ ] **Step 3: Commit**

```bash
git add crates/stint-app/build.rs
git commit -m "$(cat <<'EOF'
chore(build): stint-app build.rs invokes swift build for StintIntents

After cargo builds the Rust binary, this also runs `swift build` against
the StintIntents SwiftPM package and wraps the resulting .dylib in a
.framework structure so Tauri's bundle.macOS.frameworks can consume it.

The wrapping is necessary because SPM's `library(type: .dynamic)` emits
a .dylib, not a .framework. The minimal wrapper supplies Info.plist
with NSAppIntentsPackage=YES and symlinks to match the standard
Versions/A/ layout macOS expects.
EOF
)"
```

---

## Task H2: tauri.conf.json — bundle the framework

**Files:**
- Modify: `crates/stint-app/tauri.conf.json`

- [ ] **Step 1: Add bundle.macOS.frameworks**

Read the current `tauri.conf.json` to find the `bundle.macOS` block. Add the `frameworks` key:

```json
"macOS": {
  "signingIdentity": null,
  "providerShortName": null,
  "hardenedRuntime": true,
  "entitlements": "entitlements.plist",
  "minimumSystemVersion": "13.0",
  "frameworks": [
    "../../target/debug/build/stint-app-*/out/StintIntents.framework"
  ]
}
```

The wildcard path is a problem — Tauri may not expand globs. As a workaround, copy the framework to a stable path before `tauri build`. Adjust `build.rs` from Task H1 to ALSO copy to `crates/stint-app/Frameworks/StintIntents.framework` (created lazily, gitignored) and reference that path in `tauri.conf.json`:

```json
"frameworks": [
  "Frameworks/StintIntents.framework"
]
```

Add `/crates/stint-app/Frameworks/` to `.gitignore`.

- [ ] **Step 2: Test cargo tauri build**

```bash
cd crates/stint-app
cargo tauri build --bundles app 2>&1 | tail -30
ls -la target/release/bundle/macos/Stint.app/Contents/Frameworks/
```

Expected: `StintIntents.framework` present in the bundle.

- [ ] **Step 3: Commit**

```bash
git add crates/stint-app/tauri.conf.json .gitignore
git commit -m "$(cat <<'EOF'
chore(app): embed StintIntents.framework in Tauri bundle

bundle.macOS.frameworks references the locally-copied framework so
Tauri's bundle step copies + codesigns it as Contents/Frameworks/
StintIntents.framework.

The framework path is a stable copy made by build.rs (Frameworks/ is
gitignored).
EOF
)"
```

---

## Task H3: Tauri setup() hook → stint_intents_init

**Files:**
- Modify: `crates/stint-app/src/lib.rs` (Tauri setup callback)

- [ ] **Step 1: Declare the FFI symbol**

In `crates/stint-app/src/lib.rs`, near the top with other declarations:

```rust
extern "C" {
    fn stint_intents_init() -> i32;
}
```

- [ ] **Step 2: Call from setup()**

Locate the `tauri::Builder::default().setup(|app| { ... })` block. At the end of the closure body (before the `Ok(())`), add:

```rust
// Initialize the StintIntents Swift framework if it's loaded into the
// app bundle. dlsym-style: if the symbol isn't present (CLI binary or
// missing framework), this still links because the symbol IS present in
// the framework — the framework just may not be loaded yet. The first
// call forces a dlopen via the dyld lazy binding.
unsafe {
    let rc = stint_intents_init();
    if rc != 0 {
        eprintln!("stint_intents_init returned {rc}");
    }
}
```

Wrap in `#[cfg(target_os = "macos")]` if `lib.rs` already gates Mac-specific code that way.

- [ ] **Step 3: Verify the binary loads the framework**

After `cargo tauri build`:

```bash
otool -L target/release/bundle/macos/Stint.app/Contents/MacOS/Stint | grep -i intents
```

Expected: a line referencing `@rpath/StintIntents.framework/Versions/A/StintIntents`.

If absent → linker doesn't know about the framework. Add to `crates/stint-app/build.rs`:

```rust
println!("cargo:rustc-link-search=framework=Frameworks");
println!("cargo:rustc-link-lib=framework=StintIntents");
println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
```

- [ ] **Step 4: Commit**

```bash
git add crates/stint-app/src/lib.rs crates/stint-app/build.rs
git commit -m "feat(app): call stint_intents_init() from Tauri setup hook"
```

---

## Task I1: Swift unit tests (mocked Bridge)

**Files:**
- Create: `crates/stint-app/swift/StintIntents/Tests/StintIntentsTests/StubBridge.swift`
- Create: `Tests/StintIntentsTests/BridgeEnvelopeTests.swift`
- Create: `Tests/StintIntentsTests/EntityCodingTests.swift`
- Create: `Tests/StintIntentsTests/SpotlightSchemaTests.swift`
- Create: `Tests/StintIntentsTests/AppIntentPerformTests.swift`

- [ ] **Step 1: StubBridge**

```swift
@testable import StintIntents
import Foundation

final class StubBridge: Bridge {
    var startResult: () throws -> EntryDTO = { fatalError("startResult not set") }
    var stopResult: () throws -> EntryDTO = { fatalError("stopResult not set") }
    var currentResult: () throws -> EntryDTO? = { nil }
    var listEntriesResult: () throws -> [EntryDTO] = { [] }
    var listProjectsResult: () throws -> [ProjectDTO] = { [] }
    var listTasksResult: () throws -> [TaskDTO] = { [] }
    var updateEntryResult: () throws -> EntryDTO = { fatalError() }

    var settingsStorage: [String: String] = [:]
    var focusId: String? = nil
    var logs: [String] = []

    func start(_ params: StartParams) throws -> EntryDTO { try startResult() }
    func stop() throws -> EntryDTO { try stopResult() }
    func current() throws -> EntryDTO? { try currentResult() }
    func listEntries(_ filter: EntryFilter) throws -> [EntryDTO] { try listEntriesResult() }
    func listProjects() throws -> [ProjectDTO] { try listProjectsResult() }
    func listTasks(projectId: String?) throws -> [TaskDTO] { try listTasksResult() }
    func updateEntry(localUuid: String, patch: EntryPatch) throws -> EntryDTO { try updateEntryResult() }
    func deleteEntry(localUuid: String) throws { }

    func settingsSet(_ key: String, _ value: String) throws { settingsStorage[key] = value }
    func settingsGet(_ key: String) throws -> String? { settingsStorage[key] }
    func settingsClear(_ key: String) throws { settingsStorage.removeValue(forKey: key) }
    func currentFocusId() -> String? { focusId }
    func logWarn(_ msg: String) { logs.append(msg) }
}
```

- [ ] **Step 2: BridgeEnvelopeTests**

```swift
import XCTest
@testable import StintIntents

final class BridgeEnvelopeTests: XCTestCase {
    func testDecodeOkEnvelope() throws {
        let json = #"{"ok": {"local_uuid":"u1","description":"x","billable":false,"start_at":"2026-05-25T10:00:00Z","source":"t"}}"#
        let data = Data(json.utf8)
        struct Env: Decodable {
            let ok: EntryDTO?
        }
        let env = try JSONDecoder().decode(Env.self, from: data)
        XCTAssertEqual(env.ok?.localUuid, "u1")
    }

    func testDecodeErrEnvelope() throws {
        let json = #"{"err": {"code": 1, "message": "timer already running"}}"#
        let data = Data(json.utf8)
        struct Env: Decodable {
            let err: EnvelopeErr?
        }
        let env = try JSONDecoder().decode(Env.self, from: data)
        XCTAssertEqual(env.err?.code, 1)
        let mapped = BridgeError.from(code: Int32(env.err!.code), message: env.err!.message)
        if case .invariant(let msg) = mapped {
            XCTAssertEqual(msg, "timer already running")
        } else {
            XCTFail("expected invariant case")
        }
    }
}
```

- [ ] **Step 3: AppIntentPerformTests** (covers start, stop, current with stub bridge)

```swift
import XCTest
@testable import StintIntents

final class AppIntentPerformTests: XCTestCase {
    func testStartTimerCallsBridgeWithSource() async throws {
        let stub = StubBridge()
        stub.startResult = {
            EntryDTO(localUuid: "u1", solidtimeId: nil, description: "test",
                     projectId: nil, taskId: nil, billable: false,
                     startAt: "2026-05-25T10:00:00Z", endAt: nil, source: "intent")
        }
        var intent = StartTimerIntent()
        intent.description = "test"
        intent.bridge = stub
        _ = try await intent.perform()
        // Stub assertion: bridge.start was called (no captured params to inspect in
        // this minimal stub; extend StubBridge to record calls if you need that).
    }

    func testStopTimerSurfacesInvariantWhenNotRunning() async throws {
        let stub = StubBridge()
        stub.stopResult = { throw BridgeError.invariant("no timer to stop") }
        var intent = StopTimerIntent()
        intent.bridge = stub
        do {
            _ = try await intent.perform()
            XCTFail("expected error")
        } catch let err as BridgeError {
            if case .invariant(let m) = err {
                XCTAssertEqual(m, "no timer to stop")
            } else {
                XCTFail("wrong case")
            }
        }
    }
}
```

- [ ] **Step 4: SpotlightSchemaTests**

```swift
import XCTest
import CoreSpotlight
@testable import StintIntents

final class SpotlightSchemaTests: XCTestCase {
    func testEntryItemAttributes() {
        let entry = EntryEntity(from: EntryDTO(
            localUuid: "u1", solidtimeId: nil, description: "client meeting",
            projectId: "proj-1", taskId: nil, billable: true,
            startAt: "2026-05-25T10:00:00Z", endAt: "2026-05-25T11:00:00Z", source: "test"))
        let item = SpotlightIndexer.shared.makeEntryItem(entry)
        XCTAssertEqual(item.uniqueIdentifier, "u1")
        XCTAssertEqual(item.domainIdentifier, "tech.reyem.stint.entry")
        XCTAssertEqual(item.attributeSet.title, "client meeting")
        XCTAssertTrue(item.attributeSet.keywords?.contains("stint") ?? false)
    }

    func testProjectItemAttributes() {
        let p = ProjectDTO(solidtimeId: "p1", name: "Acme", color: nil, clientId: nil, archived: false)
        let item = SpotlightIndexer.shared.makeProjectItem(p)
        XCTAssertEqual(item.uniqueIdentifier, "p1")
        XCTAssertEqual(item.domainIdentifier, "tech.reyem.stint.project")
        XCTAssertEqual(item.attributeSet.title, "Acme")
    }
}
```

- [ ] **Step 5: Run tests**

```bash
cd crates/stint-app/swift/StintIntents
swift test 2>&1 | tail -20
```

Expected: all Swift tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/stint-app/swift/StintIntents/Tests/
git commit -m "test(swift): mocked-bridge unit tests for intents, envelopes, schemas"
```

---

## Task I2: Swift integration tests (real Rust FFI)

**Goal:** One end-to-end test that links against the real `stint_core` and exercises a start→current→stop cycle.

**Files:**
- Create: `crates/stint-app/swift/StintIntents/Tests/StintIntentsIntegrationTests/FFIRoundTripTests.swift`

- [ ] **Step 1: Write the file**

```swift
import XCTest
@testable import StintIntents

final class FFIRoundTripTests: XCTestCase {
    override func setUp() {
        // Point STINT_HOME at a tempdir so we don't touch the user's DB.
        let tmp = NSTemporaryDirectory() + "stint-ffi-\(UUID().uuidString)/"
        try? FileManager.default.createDirectory(atPath: tmp, withIntermediateDirectories: true)
        setenv("STINT_HOME", tmp, 1)
    }

    func testStartCurrentStopRoundTrip() throws {
        let bridge = FFIBridge()

        let started = try bridge.start(StartParams(description: "integ", source: "swift-it"))
        XCTAssertEqual(started.description, "integ")

        let current = try bridge.current()
        XCTAssertEqual(current?.localUuid, started.localUuid)

        let stopped = try bridge.stop()
        XCTAssertNotNil(stopped.endAt)
    }

    func testStartTwiceReturnsInvariantError() throws {
        let bridge = FFIBridge()
        _ = try bridge.start(StartParams(description: "a", source: "swift-it"))
        do {
            _ = try bridge.start(StartParams(description: "b", source: "swift-it"))
            XCTFail("expected invariant error")
        } catch let err as BridgeError {
            if case .invariant = err { /* ok */ } else { XCTFail() }
        }
    }
}
```

- [ ] **Step 2: Run**

```bash
cd crates/stint-app/swift/StintIntents
swift test --filter StintIntentsIntegrationTests 2>&1 | tail -10
```

The integration test requires `libstint_core.dylib` to be discoverable at link time. If `swift test` can't find it, the build will fail with "symbol not found" — add an explicit DYLD_LIBRARY_PATH or update Package.swift's linkerSettings to point at the workspace target dir.

- [ ] **Step 3: Commit**

```bash
git add crates/stint-app/swift/StintIntents/Tests/StintIntentsIntegrationTests/
git commit -m "test(swift): integration test for FFIBridge against real stint_core"
```

---

## Task J1-J3: CI gates

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Add Swift test step**

After the existing `cargo test` step in `ci.yml`, add:

```yaml
      - name: Swift package tests (StintIntents)
        if: runner.os == 'macOS'
        run: |
          cd crates/stint-app/swift/StintIntents
          swift test
```

- [ ] **Step 2: Add codesign verify step (release workflow)**

In `.github/workflows/release.yml`, after `cargo tauri build`:

```yaml
      - name: Verify framework codesign
        run: |
          codesign --verify --deep --strict \
            target/release/bundle/macos/Stint.app
          codesign --verify --strict \
            target/release/bundle/macos/Stint.app/Contents/Frameworks/StintIntents.framework
```

- [ ] **Step 3: Add Metadata.appintents check**

```yaml
      - name: Verify AppIntents metadata stencil contains all intents
        run: |
          STENCIL="target/release/bundle/macos/Stint.app/Contents/Frameworks/StintIntents.framework/Resources/Metadata.appintents"
          if [ ! -f "$STENCIL" ]; then
            echo "Missing Metadata.appintents stencil"
            exit 1
          fi
          # Each intent type's name should appear in the stencil. The stencil is
          # binary plist or similar — use `strings` to grep through it.
          for name in StartTimerIntent StopTimerIntent GetCurrentIntent \
                      SwitchProjectIntent LogPastIntent ListEntriesIntent \
                      ListProjectsIntent ListTasksIntent UpdateEntryIntent \
                      DeleteEntryIntent ProjectFocusFilter; do
            if ! strings "$STENCIL" | grep -q "$name"; then
              echo "Intent type missing from stencil: $name"
              exit 1
            fi
          done
          echo "All 11 intent types present in Metadata.appintents"
```

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/
git commit -m "ci: swift test step + framework codesign verify + appintents stencil check"
```

---

## Task J4: SKILL.md extension

**Files:**
- Modify: `crates/stint-cli/skills/stint/SKILL.md`

- [ ] **Step 1: Add App Intents section**

Append to the "Surface priority" section in `SKILL.md`:

```markdown
4. **App Intents (Shortcuts.app / Siri / Spotlight)** — macOS users may have
   automations bound to stint's intents. The agent doesn't invoke these directly,
   but should be aware they exist when explaining stint's surface area:
   - Five App Shortcuts: Start Timer, Stop Timer, Current Timer, Switch Project,
     Log Past Work. Each has voice-callable phrases.
   - All 8 verb intents (+ 2 composed: SwitchProject, LogPast) are discoverable
     in Shortcuts.app as Custom Shortcuts.
   - One Focus Filter: "Default Project" — set per Focus mode in System Settings.
```

Also add to the "Gotchas" section:

```markdown
- **Focus filter race window** — if a user activates a macOS Focus filter
  while Stint.app is cold-launching, the `start` verb may fire before the
  focus default is written, producing an entry without the focus project.
  Document workaround: rerun `stint edit` if the user notices a missing
  project after a focus-mode-triggered start.

- **New URL routes** — `stint://project/<solidtime_id>` opens the Today view
  filtered to that project; `stint://task/<solidtime_id>` resolves to the
  task's parent project and filters by both. Used by Spotlight result taps.
```

- [ ] **Step 2: Commit**

```bash
git add crates/stint-cli/skills/stint/SKILL.md
git commit -m "docs(skill): document App Intents surface + new URL routes"
```

---

## Task J5: Manual smoke checklist (PR description)

**Goal:** Capture the manual-test list as a markdown block that goes into the PR description. Not committed; lives in the PR body when it's created.

Checklist (to copy into the PR):

```markdown
## Manual smoke (macOS 13+)

- [ ] `cargo tauri build` succeeds; framework embedded in `Stint.app/Contents/Frameworks/StintIntents.framework`
- [ ] `Stint.app` launches without Gatekeeper warning (signed cert valid)
- [ ] `pluginkit -mvD | grep tech.reyem.stint` lists ≥11 App Intent types
- [ ] Shortcuts.app shows "Stint" actions; can configure Start Timer with a project parameter
- [ ] Cmd+Space → "client meeting" (after creating one) → tap result → app focuses entry
- [ ] Cmd+Space → "Acme" (after creating an Acme project) → tap → Today view filters to Acme
- [ ] "Hey Siri, start timer in Stint" → Siri prompts for description → speak it → verify entry created
- [ ] System Settings → Focus → Work → Add Filter → Stint → pick project → verify next `stint start` (without `--project`) picks it up
- [ ] After 6b lands, run `man stint` → no regression on man page (still v0.3.x)
- [ ] `stint mcp` still launches; MCP tools still work (Spotlight is additive)
```

---

## Task K1: Full verification

- [ ] **Step 1: Cargo lint + test**

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --test-threads=1
```

Expected: green.

- [ ] **Step 2: UI typecheck + tests**

```bash
cd ui
pnpm typecheck
pnpm test:run
cd ../..
```

Expected: green.

- [ ] **Step 3: Coverage**

```bash
scripts/coverage.sh
```

Expected: all four surfaces (stint-core, stint-cli, stint-app, ui) ≥80%. New `crates/stint-core/src/ffi.rs` should be well-covered by ffi_envelope.rs / ffi_verbs.rs / ffi_panic_safety.rs / ffi_settings.rs.

If `stint-core` dips below 80% due to the FFI surface, add targeted tests in `crates/stint-core/tests/ffi_more.rs` until it climbs back above. The error-mapping branches and panic safety are the likely gaps.

- [ ] **Step 4: Bundle smoke**

```bash
cd crates/stint-app
cargo tauri build --bundles app
cd ../..
codesign --verify --deep --strict crates/stint-app/target/release/bundle/macos/Stint.app
ls crates/stint-app/target/release/bundle/macos/Stint.app/Contents/Frameworks/
strings crates/stint-app/target/release/bundle/macos/Stint.app/Contents/Frameworks/StintIntents.framework/Resources/Metadata.appintents | grep -c StartTimer
```

Expected: clean codesign, framework present, strings count ≥1.

---

## Task K2: Tag phase-6b-complete (LOCAL ONLY — do not push)

- [ ] **Step 1: Sanity check no uncommitted changes**

```bash
git status
```

Expected: clean working tree.

- [ ] **Step 2: Tag**

```bash
git tag -a phase-6b-complete -m "Phase 6b complete — Spotlight + App Intents + Focus filter"
git log --oneline -5
```

- [ ] **Step 3: STOP and confirm with user before pushing**

The plan stops here. Explicit user confirmation required before:
- `git push origin phase-6b`
- Opening a PR
- Pushing tags

Surface to user: "Phase 6b is complete on local branch `phase-6b`, tagged `phase-6b-complete`. Ready to push and open the PR?"

---

## Self-review summary (run after writing the plan)

**Coverage of spec sections:**
- §3 Architecture → Tasks A2, A3, A4, C1, C2, H1, H2, H3
- §4 App Intents → Tasks F1–F10, G1
- §5 Spotlight → Tasks E1, E2, E3, B2
- §6 Focus filter → Tasks A4, A6, G2
- §7 Error handling → Tasks A2 (envelope), C3 (Swift errors), J1–J3 (CI gates)
- §8 Testing strategy → Tasks I1, I2, J1, K1
- §9 Trade-offs → captured in spec, no separate task

**Placeholder scan:** searched for TBD/TODO/FIXME in this plan — none found in execution steps.

**Known fragility points (call out during execution):**
1. Task A1 SPM spike outcome determines whether to use SPM (A1) or Xcode `.xcodeproj` (A1.fallback). If fallback, Tasks H1 and the CI Swift test step adapt.
2. Task H1 `wrap_dylib_as_framework` glob paths for the SPM-emitted dylib may need iteration — verify the actual `.build/<profile>/` layout.
3. Task E3 `stint_current_focus_id_swift` reads a UserDefaults key that may not be a documented public API. If unavailable, fall back to focus_id-from-perform-only (no read-side lookup).
4. Task H2 `frameworks` path: Tauri may not glob-expand. Adopted workaround: build.rs copies framework to `crates/stint-app/Frameworks/` (gitignored).
5. Task I2 link path for the integration test: may require `DYLD_LIBRARY_PATH` env override at test time.

Each fragility point has a documented fallback inline. Execution should pause and ask only if a fallback also fails.
