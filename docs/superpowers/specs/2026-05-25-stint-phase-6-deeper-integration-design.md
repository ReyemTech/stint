# stint — Phase 6: Deeper macOS integration (spec)

Extend stint beyond CLI/GUI/MCP/HTTP into the macOS shell itself — App Intents (Shortcuts + Siri + Focus filters), Core Spotlight (CSSearchableIndex + NSUserActivity), Raycast/Alfred surfaces, WidgetKit, and idle detection. Built on top of the Phase 6a verbs façade.

- **Status:** Confirmed 2026-05-25. **Shipped 2026-05-26 as foundation-only — see §1.5 for what's actually live vs deferred.**
- **Predecessors:** Phase 6a (verbs façade, MCP, HTTP API, `stint://` URL scheme, `stint skill install`, man page — shipped)
- **Decomposition:** This phase splits into two sub-phases.
  - **6b** — Core Spotlight + App Intents (Shortcuts / Siri / Focus filters). Detailed below.
  - **6c** — Raycast extension + Alfred workflow + WidgetKit + idle detection. Outlined here, full spec to be written when 6b ships.

## 1.5 What actually shipped (2026-05-26)

**This is a partial ship — Spotlight works end-to-end, Siri/Shortcuts/Focus-filter discovery doesn't.** After extended debugging — switching from embedded framework to static-link and back, ad-hoc and Developer-ID signing, full notarization, app-level Metadata.appintents stencil — Apple's App Intents indexer (`siriactionsd`/`assistantd`/Shortcuts.app) remains silent on our bundle. We accept that the path from "intents in a SwiftPM target linked into a non-Xcode-driven app" to "macOS Siri/Shortcuts discovery" has an undocumented gap we couldn't isolate from CLI. Spotlight's NSUserActivity + CSSearchableIndex surfaces, on the other hand, **do work** once the right architecture was found.

| Surface | Status | Notes |
|---|---|---|
| Rust FFI bridge (`stint_core::ffi`) | ✅ shipped | 8 verb wrappers, settings, log forwarder, focus id, notify_indexer hook — all tested via cargo tests |
| `stint://` URL routes (entry / project / task) | ✅ shipped | Tauri deep-link handler routes them to the SolidJS UI; entry route looks up the date and emits `/today?entry=<uuid>&date=<YYYY-MM-DD>` for the Today view to highlight |
| Entry-row scroll + amber-pulse highlight on deep-link | ✅ shipped | Today.tsx reads `?entry=` via useSearchParams; EntryRow scrolls into view + adds a 2.5s ring-amber-400 pulse |
| `focus.default_project` fallback in `verbs::start` | ✅ shipped | Applies the focus filter's persisted default; reconciled against current focus id |
| Swift StintIntents framework (dynamic) | ✅ shipped | Embedded at `Contents/Frameworks/StintIntents.framework`; loads at app launch via `-needed_framework`; `stint_intents_init` runs (verified in production log) |
| `NSUserActivity` for the running entry | ✅ shipped | Activity carries `webpageURL = stint://entry/<uuid>` so taps from Spotlight's live-activity tile route correctly |
| `CSSearchableIndex` entries / projects / tasks | ✅ shipped | Indexed items carry `attributeSet.url` set to the matching `stint://` route; taps route through the deep-link handler |
| **App Intents discovery by Siri / Shortcuts.app** | ❌ **deferred** | Apple's indexer remains silent on our bundle. Likely requires using Apple's App Intents Extension (.appex) target template, which Tauri can't currently produce |
| **`ProjectFocusFilter` in System Settings → Focus** | ❌ **deferred** | Same root cause as above |

**What this is good for:** Spotlight integration works — Cmd+Space → entry description → tap → opens Stint focused on that entry. URL scheme additions, focus-fallback infrastructure, and the FFI bridge are all live.

**What this is not good for:** anything that requires Siri voice activation or Shortcuts.app gallery discovery.

**Re-enabling the still-deferred Siri/Shortcuts surfaces** is a follow-up that should:
1. Add a real `.xcodeproj` (or `.appex` extension target) that produces a proper App Intents Extension bundle under `Contents/PlugIns/`.
2. Move the existing Swift intent types into that target unchanged.
3. The Rust FFI surface + URL scheme already in place — no Rust changes needed.

The 6c scope (Raycast / Alfred / WidgetKit / idle) is unaffected by the deferral: those surfaces don't go through Apple's intent indexer.

## 1. Goal

Make stint feel like a first-class macOS citizen by exposing the existing verbs through the system surfaces a macOS power user expects:

- Cmd+Space → "client meeting" → tap → open the entry in stint
- "Hey Siri, start tracking 'writing tests' in Stint" → timer running
- "When Work focus is on, default new timers to my Work project" → no manual project switch
- A Stint widget on the desktop showing the running timer and total hours today (6c)
- Raycast / Alfred ⌥-Space → fuzzy-match stint actions (6c)
- "You've been idle for 10 minutes — was that part of your timer?" (6c)

All of this consumes the Phase 6a verbs façade. Zero new business logic in 6b — only transport adapters into Apple's frameworks.

## 2. Scope

### 2.1 In scope for 6b

- **App Intents** — `AppIntent` types covering all 8 verbs (Custom Shortcuts), plus an `AppShortcutsProvider` curating 5 of them as App Shortcuts (voice / Spotlight quick-actions).
- **Core Spotlight** — `CSSearchableIndex` for entries + projects + tasks (three distinct domain identifiers). `NSUserActivity` for the currently running entry.
- **Focus filters** — one filter target: default project for new timers per Focus mode.
- **Swift packaging** — a Swift Package at `crates/stint-app/swift/StintIntents/` produces `StintIntents.framework`, embedded into the Tauri-built `Stint.app/Contents/Frameworks/`.
- **FFI bridge** — bidirectional. Rust exposes `extern "C"` verb wrappers; Swift exposes `@_cdecl` indexer-notify symbols looked up via `dlsym`.
- **URL scheme additions** — `stint://project/<id>` and `stint://task/<id>` routes for Spotlight taps.

### 2.2 In scope for 6c (outlined only)

- **Raycast extension** — TypeScript extension talking to the verbs via `stint --json` subprocess (CLI ships with the cask).
- **Alfred workflow** — equivalent to Raycast, distributed as a `.alfredworkflow` bundle.
- **WidgetKit widget** — small/medium widget showing running timer, today's totals, project breakdown. Built as a Widget Extension target in the same SPM workspace.
- **Idle detection** — macOS `CGEventSourceSecondsSinceLastEventType` polling in the GUI process. On detected idle > threshold, prompt user to discard or keep the idle minutes.

6c uses the same verbs façade and the same FFI bridge 6b establishes, so the architectural work in 6b carries through.

### 2.3 Out of scope

- **MAS (Mac App Store) submission** — Phase 4.5.
- **iOS / iPadOS targets** — separate effort; would need a re-architecture for non-macOS data sync.
- **Localization beyond `en`** — the `.xcstrings` file structure is set up to accept future translations; no other locales shipped in 6b.
- **Apple Intelligence integrations** (writing tools on entry descriptions, smart suggestions) — too new, API surface unstable.
- **Multiple Focus filter targets** — only default project in 6b. Billable defaults and Solidtime org switching are explicitly deferred.

## 3. Architecture

### 3.1 Process model

Single `Stint` binary. `StintIntents.framework` is dynamically loaded from `Contents/Frameworks/` at first FFI symbol reference. The Swift runtime loads, App Intents reflection discovers the types via the framework's `Info.plist` and `Metadata.appintents` stencil generated by SPM at build time.

The same `stint-core` crate is also consumed by `stint-cli`, which never loads the framework. The Rust→Swift indexer-notify call is resolved via `dlsym` and no-ops when the symbol is absent — `stint-cli` stays Spotlight-unaware.

### 3.2 New artifacts

```
crates/stint-app/
  swift/
    StintIntents/
      Package.swift                        # SPM manifest
      Sources/StintIntents/
        Bridge.swift                       # FFI declarations + @_cdecl exports
        Intents/
          StartTimerIntent.swift
          StopTimerIntent.swift
          GetCurrentIntent.swift
          ListEntriesIntent.swift
          ListProjectsIntent.swift
          ListTasksIntent.swift
          UpdateEntryIntent.swift
          DeleteEntryIntent.swift
          SwitchProjectIntent.swift
          LogPastIntent.swift
        Shortcuts/
          StintAppShortcutsProvider.swift  # the 5 curated App Shortcuts
          PhraseStrings.xcstrings          # phrase localization (en seeded)
        Entities/
          EntryEntity.swift                # AppEntity + IndexedEntity
          ProjectEntity.swift
          TaskEntity.swift
          EntryQuery.swift                 # EntityQuery + EntityStringQuery
          ProjectQuery.swift
          TaskQuery.swift
        Spotlight/
          SpotlightIndexer.swift           # CSSearchableIndex bulk + delta
          ActivityTracker.swift            # NSUserActivity for running entry
        Focus/
          ProjectFocusFilter.swift         # SetFocusFilterIntent
        Errors/
          BridgeError.swift                # IntentError + envelope decode
      Tests/StintIntentsTests/             # unit tests (mocked bridge)
      Tests/StintIntentsIntegrationTests/  # links real stint_core static lib
  build.rs                                 # extended: `swift build` + copy framework

crates/stint-core/
  src/
    ffi.rs                                 # extern "C" verb wrappers + envelope
    url_scheme.rs                          # extended: OpenProject, OpenTask
  include/
    stint_core.h                           # C header for Swift bridging
```

### 3.3 Bundle layout (post-build)

```
Stint.app/
  Contents/
    MacOS/Stint                    # Rust binary with FFI symbols
    Frameworks/
      StintIntents.framework/
        StintIntents               # Swift dylib
        Info.plist
        Resources/
          Metadata.appintents      # generated by SPM at build time
          PhraseStrings.lproj/
    Resources/
      man/man1/stint.1             # existing
```

Tauri's `bundle.macOS.frameworks` in `tauri.conf.json` lists the SPM-built framework path. Tauri's bundle step copies + codesigns it as part of the standard release flow.

### 3.4 IPC channels

| Direction | Channel | Used for |
|---|---|---|
| Swift → Rust | `extern "C"` FFI | App Intent `perform()` — needs return values |
| Rust → Swift | `@_cdecl` via `dlsym` | Spotlight index delta on verb mutation |
| Swift → System | `stint://...` URL | "Open the GUI focused on X" Custom Shortcuts |
| System → Swift | NSUserActivity / `CSSearchableItem` tap | Spotlight result tap routes through `stint://entry/<uuid>` to existing deep-link handler |

### 3.5 Build flow

```
cargo build -p stint-app
  └─→ stint-app/build.rs
        ├─→ swift build --product StintIntents -c <profile>
        │     └─→ produces StintIntents.framework (SPM with xcodebuild post-step)
        └─→ copies framework to OUT_DIR

cargo tauri build
  └─→ tauri reads bundle.macOS.frameworks → embeds + codesigns
```

The 30-minute SPM spike (first execution task) verifies `swift build` produces a usable framework with the App Intents metadata stencil correctly generated. If that fails, fall back to an Xcode `.xcodeproj` driven by `xcodebuild` — rest of design unchanged.

## 4. App Intents surface

### 4.1 App Shortcuts (curated, public phrase contract)

| # | Intent | Phrases | Parameters | Returns |
|---|---|---|---|---|
| 1 | `StartTimerIntent` | "Start timer in Stint", "Start tracking in Stint", "Start ${project} in Stint" | optional `project: ProjectEntity`, prompts for `description` | dialog: "Tracking '${desc}' on ${project}." |
| 2 | `StopTimerIntent` | "Stop Stint timer", "Stop tracking in Stint" | none | dialog: "Stopped. ${duration} on ${project}." |
| 3 | `GetCurrentIntent` | "What am I tracking in Stint", "Show current Stint timer" | none | `EntryEntity` + dialog |
| 4 | `SwitchProjectIntent` | "Switch to ${project} in Stint" | required `project: ProjectEntity` | dialog: "Switched to ${project}." |
| 5 | `LogPastIntent` | "Log past ${duration} in Stint", "Log last meeting in Stint" | required `duration: Measurement<UnitDuration>`, optional `project`, optional `description` | dialog: "Logged ${duration} on ${project}." |

**Phrase strings are a public contract.** Once shipped, renaming them breaks users' voice shortcuts. Strings live in `PhraseStrings.xcstrings`.

### 4.2 Custom Shortcuts (full verb surface)

All 8 verbs exposed as `AppIntent` types, discoverable in Shortcuts.app. The five App Shortcut intents above double as Custom Shortcuts. Three additional Custom-only intents:

- `ListEntriesIntent` — `since?`, `until?`, `project?`, `limit?` → `[EntryEntity]`. Chainable in Shortcuts pipelines.
- `ListProjectsIntent` → `[ProjectEntity]`.
- `ListTasksIntent` — `project: ProjectEntity` → `[TaskEntity]`.
- `UpdateEntryIntent` — `entry: EntryEntity`, optional `description`, `project`, `task`, `billable`, `startAt`, `endAt` (per `EntryPatch` semantics) → `EntryEntity`.
- `DeleteEntryIntent` — `entry: EntryEntity` → void.

Each takes a `Bridge` (protocol) via `init()` with default `FFIBridge.shared` for production and `StubBridge` injection in unit tests.

### 4.3 Entities

| Entity | `id` | `title` | `subtitle` | `image` |
|---|---|---|---|---|
| `EntryEntity` | `local_uuid` | description | `${date} · ${duration} · ${project_name}` | project color swatch |
| `ProjectEntity` | `solidtime_id` | project name | `Project${client?: " · " + client_name}` | color swatch |
| `TaskEntity` | task UUID | task name | `Task in ${project_name}` | parent project color |

`EntryQuery: EntityStringQuery` allows fuzzy-string matching ("entry about lunch") for parameter resolution. `ProjectQuery: EntityQuery` and `TaskQuery: EntityQuery` provide enumeration via the bridge.

Each entity declares `@Property` annotations on filterable fields (`billable: Bool`, `duration: Measurement<UnitDuration>`, `startAt: Date`, `endAt: Date?`, `project: ProjectEntity?`) so Shortcuts can compose filters and computations.

### 4.4 Composed-intent semantics

- **`SwitchProjectIntent`** = `stop` (if running) → `start` with same description + new project. Errors with "No timer to switch from." if no current entry.
- **`LogPastIntent`** = `start { start_at: now - duration, … }` → `stop`. Reuses existing semantics, no new "retroactive entry" verb needed.

## 5. Spotlight indexing

### 5.1 Domain identifiers

| Domain | Source | Title | Subtitle | Keywords | Tap → |
|---|---|---|---|---|---|
| `tech.reyem.stint.entry` | `local_uuid` | description | `${date} · ${duration} · ${project_name}` | project name, task name, "stint" | `stint://entry/<uuid>` |
| `tech.reyem.stint.project` | `solidtime_id` | project name | `Project${client?: " · " + client_name}` | project name, client name | `stint://project/<id>` (new) |
| `tech.reyem.stint.task` | task UUID | task name | `Task in ${project_name}` | task name, parent project name | `stint://task/<id>` (new) |

`CSSearchableItemAttributeSet.thumbnailData` is a 16×16 PNG generated on the fly from the project color. Generated once per project per session and cached in a `[String: Data]` dictionary keyed by hex color.

### 5.2 `NSUserActivity` for the running entry

```swift
activityType            = "tech.reyem.stint.tracking"
title                   = "Tracking: \(description)"
userInfo                = ["uuid": local_uuid]
isEligibleForSearch     = true
isEligibleForHandoff    = true
isEligibleForPrediction = true
```

Activated on `start`, mutated on `update_entry` if the running entry's description changes, invalidated on `stop`. Surfaces at the top of Spotlight as a live-activity card.

### 5.3 Indexer lifecycle

```
App launch (Tauri setup())
  └─→ stint_intents_init() [Rust → Swift FFI]
        ├─→ Task.detached(priority: .background) {
        │       SpotlightIndexer.bulkRefresh()
        │         ├─→ FFI list_entries  → upsert all entry items
        │         ├─→ FFI list_projects → upsert all project items
        │         └─→ FFI list_tasks    → upsert all task items
        │   }
        └─→ ActivityTracker.activate()
              └─→ FFI current → register NSUserActivity if running

Verb mutation (after successful store write, before sync enqueue)
  └─→ stint_core::ffi::notify_indexer(kind, payload_json)
        └─→ cached dlsym("swift_indexer_notify") → call (or no-op)
              └─→ SpotlightIndexer.delta(kind, payload)
                    ├─→ EntryStarted/Updated: upsert + ActivityTracker.activate
                    ├─→ EntryStopped:         upsert + ActivityTracker.invalidate
                    ├─→ EntryDeleted:         deleteSearchableItems([uuid])
                    └─→ ProjectsReplaced / TasksReplaced (from pull_worker): re-bulk that slice
```

The bulk refresh is dispatched off the setup() critical path. First-launch Spotlight results may be stale for up to ~1-2 seconds after launch — accepted.

### 5.4 Index consistency model

macOS is the source of truth. No `last_indexed_at` columns in SQLite. Bulk reindex on every launch uses `indexSearchableItems` with upsert-on-unique-identifier semantics — no delete-first needed. Explicit deletes via `deleteSearchableItems(withIdentifiers:)`.

**Accepted edge case:** if entries are deleted while Stint.app is not running, the index could orphan. Currently impossible — every delete path (CLI, MCP, HTTP) routes through `verbs::delete_entry` which calls `notify_indexer` synchronously, and the GUI must be running for HTTP. If this changes in a future phase (e.g., a background sync worker that deletes adopted entries), a GC pass that reconciles against SQLite on launch becomes necessary.

### 5.5 New URL routes

```rust
// crates/stint-core/src/url_scheme.rs
pub enum Action {
    // existing
    Start { ... }, Stop, OpenEntry { local_uuid }, Current,
    // new in 6b
    OpenProject { project_id: String },
    OpenTask    { task_id:    String },
}
```

`OpenProject` navigates to `/today?project=<id>`. `OpenTask` resolves task → project_id via `verbs::list_tasks` then navigates to `/today?project=<pid>&task=<tid>`.

## 6. Focus filters

### 6.1 Filter target

One: **default project for new timers per Focus mode.**

### 6.2 Swift type

```swift
struct ProjectFocusFilter: SetFocusFilterIntent {
    static let title: LocalizedStringResource = "Default Project"
    static let description: IntentDescription = "Set a default project for new Stint timers while this focus is on."

    @Parameter(title: "Project") var project: ProjectEntity

    func perform() async throws -> some IntentResult {
        // OS calls perform() on every focus activation that has this filter
        // configured. It does NOT call perform() on deactivation. We store the
        // currently-active focus identifier alongside the project so the
        // start-verb path can reconcile.
        let focusId = INFocusStatusCenter.default.focusStatus.activity?.identifier ?? ""
        let payload = "\(focusId)\t\(project.id)"
        let rc = stint_settings_set("focus.default_project", payload)
        guard rc == 0 else { throw BridgeError.internal("settings_set failed") }
        return .result()
    }
}
```

**Deactivation reconciliation.** The OS does not invoke `perform()` on focus deactivation. To detect "this filter is no longer active" we store `(focus_id, project_id)` as a tab-separated string. The Rust `verbs::start` fallback path reads both, queries the current macOS focus identifier via a small FFI (`stint_current_focus_id() -> *mut c_char`), and applies the stored `project_id` only if the focus IDs match. If they differ, the stored value is stale and ignored. This is simpler than registering `NSWorkspace` focus observers from Swift and dealing with their lifetime.

### 6.3 Fallback semantics in `verbs::start`

```rust
pub fn start(store: &Store, params: StartParams) -> Result<EntryView> {
    let project_id = params.project_id.or_else(|| {
        // Read the (focus_id, project_id) pair stored by ProjectFocusFilter.
        // Reconcile against the currently active focus — ignore if stale.
        let raw = store.settings_get("focus.default_project").ok().flatten()?;
        let (stored_focus, project_id) = raw.split_once('\t')?;
        let current_focus = focus::current_id();  // shells out to dlsym'd Swift fn
        (current_focus.as_deref() == Some(stored_focus)).then(|| project_id.to_string())
    });
    // existing logic with (possibly defaulted) project_id
}
```

Applies uniformly to CLI, MCP, HTTP, GUI, and App Intents — anywhere a start is initiated without an explicit project.

### 6.4 Edge cases

- **Running timer when focus changes** — untouched. Focus default applies only to new starts. Stopping+restarting would corrupt the user's current tracking.
- **CLI start immediately after focus activation while app is cold-launching** — ~200ms race window where settings write may not have landed. Worst case: entry created without the focus default; fixable via `stint edit`. Documented in `SKILL.md`.

## 7. Error handling

### 7.1 Envelope contract

Every FFI verb returns a JSON envelope:

```
{ "ok":  <T> }
| { "err": { "code": <int>, "message": "<str>" } }
```

Codes are a stable contract (never renumber):

| Code | Variant | Surface |
|---|---|---|
| 0 | success | (the verb's success dialog) |
| 1 | `Invariant` (e.g., timer already running) | message verbatim |
| 2 | `NotFound` (project / entry lookup miss) | message verbatim |
| 3 | `Conflict` (sync overlap) | "That conflicts with an existing entry." |
| 4 | `Serialization` | "Couldn't read the request." |
| 99 | `Internal` | "Stint hit an internal error. Check the app." |
| -1 | `Panic` (caught via `catch_unwind`) | "Stint encountered an unexpected error." |
| -2 | `Misuse` (null out_json) | unreachable from Swift bridge |

### 7.2 Panic safety

Every FFI wrapper body runs inside `std::panic::catch_unwind`. A caught panic becomes a `-1` envelope with the panic message. Without this, a panic across the C ABI is undefined behavior. Test: `crates/stint-core/tests/ffi_panic_safety.rs` forces a panic and asserts the envelope.

### 7.3 Sync errors stay local-first

Solidtime upload failures are handled by the existing async `sync_worker`. They **never** surface to App Intents — the intent's `perform()` only knows whether the local write succeeded. The user gets "Started timer." even if Solidtime is down; the GUI's existing `SyncErrorBanner` surfaces the failure on next launch.

### 7.4 Spotlight / NSUserActivity are best-effort

Spotlight write failures are logged via `stint_log_warn` (a small FFI surface that funnels into the existing `tracing` subscriber) and never propagate to the caller. Next `bulkRefresh()` reconciles.

### 7.5 Framework load-time failures

| Failure | Detection | Behavior |
|---|---|---|
| Framework missing from bundle | First FFI call dlopens implicitly; symbol unresolved at link time → launch fails | CI gate catches this before release |
| Codesign mismatch | Gatekeeper blocks launch | CI: `codesign --verify --deep --strict Stint.app` |
| App Intents not registered (metadata stencil broken) | Intents don't appear in Shortcuts.app | CI: parse `Stint.app/Contents/Frameworks/StintIntents.framework/Resources/Metadata.appintents` and assert ≥11 intent types (8 verb intents + 2 composed + `ProjectFocusFilter`) |
| `swift_indexer_notify` symbol missing in CLI builds | `dlsym` returns null | No-op; CLI stays Spotlight-unaware |

### 7.6 Concurrency

The Store is `Arc<Mutex>`-backed. Concurrent FFI calls from Swift+CLI+MCP+HTTP serialize through the same mutex. No new concurrency primitives in 6b.

## 8. Testing strategy

Five layers:

| Layer | Location | Run via | Counted toward coverage |
|---|---|---|---|
| Rust FFI wrappers | `crates/stint-core/tests/ffi*.rs` | `cargo test` | Yes (stint-core) |
| Swift unit tests (mocked bridge) | `crates/stint-app/swift/StintIntents/Tests/StintIntentsTests/` | `swift test` | Tracked separately (≥80% local) |
| Swift integration (real Rust FFI) | `Tests/StintIntentsIntegrationTests/` | `swift test` | Tracked separately |
| Bundle integration (CI-only smoke) | `.github/workflows/ci.yml` | `codesign --verify` + `pluginkit -mvD` | N/A |
| Manual smoke checklist | PR description | Reviewer-driven | N/A |

Swift coverage is **not** merged into `scripts/coverage.sh` in 6b — deferred to a follow-up chore. Local discipline: ≥80% line coverage on `Sources/StintIntents/` via `swift test --enable-code-coverage`.

What's not tested:
- Real Spotlight search results (macOS indexing pipeline timing is non-deterministic in CI).
- Siri voice recognition (impossible to automate).
- `NSUserActivity` handoff (requires two-Mac setup).

These are accepted manual-smoke items.

## 9. Trade-offs and deferred work

| Decision | Trade-off | Deferred alternative |
|---|---|---|
| SPM-built framework (not Xcode `.xcodeproj`) | Cleaner manifest; risk in `.xcstrings` phrase generation | Xcode project fallback if 30-min spike fails |
| FFI + URL scheme hybrid | Two channels to maintain | Single-channel HTTP-only — adds latency and a port-discovery step |
| `dlsym` lookup for indexer-notify | CLI binary stays unaware of Spotlight | Static linking — would force CLI to ship framework or fail to link |
| App Intents in Custom + 5 App Shortcuts | Phrase strings are a public contract | Custom-only — invisible to non-Shortcuts users |
| Comprehensive Spotlight (entry+project+task) | +1 day of Swift code | Entry-only — would still need EntityQuery for parameter resolution |
| One Focus filter (default project) | Limited scope | Add billable / org-switch filters in 6b.1 or 6c (30 LoC each) |
| No GC pass on the Spotlight index | Assumes deletes always run through `notify_indexer` | Add reconcile-on-launch sweep if invariant breaks |
| Swift coverage not in unified report | Local-only discipline in 6b | Merge into `scripts/coverage.sh` as a follow-up chore |
| Launch-time bulk reindex on background queue | Stale results for ~1-2s after launch | Synchronous reindex — would block app launch UI |

## 10. Implementation order (preview)

The plan doc (`docs/superpowers/plans/2026-05-25-stint-phase-6b-spotlight-app-intents.md`) will sequence the work. High-level order:

1. **SPM spike** (30 min) — produce a minimal `StintIntents.framework` with one stub App Shortcut; verify `pluginkit -mvD` sees it.
2. **Rust FFI surface** — `crates/stint-core/src/ffi.rs` with envelope + 8 verb wrappers + settings get/set + log forwarder + panic-safety test.
3. **C header** — hand-written `stint_core.h` consumed by Swift bridging header.
4. **Swift package scaffold** — `Package.swift`, `Bridge.swift` with extern declarations + `Bridge` protocol + `FFIBridge` impl + `StubBridge` test impl.
5. **Entities** — `EntryEntity`, `ProjectEntity`, `TaskEntity` + their `EntityQuery` types.
6. **Spotlight** — `SpotlightIndexer`, `ActivityTracker`. Unit tests against `CSSearchableItemAttributeSet` shape.
7. **App Intents** — 10 intent types (5 App Shortcuts × double-duty + 3 list intents + update + delete). One file per intent, mocked-bridge unit test each.
8. **App Shortcuts provider** — `StintAppShortcutsProvider` + `PhraseStrings.xcstrings`.
9. **Focus filter** — `ProjectFocusFilter` + `stint_settings_set/clear` FFI + fallback in `verbs::start`.
10. **URL scheme extension** — `OpenProject`, `OpenTask` actions + Tauri deep-link routing + UI navigation.
11. **Tauri integration** — `stint-app/build.rs` runs `swift build` + copies framework; `tauri.conf.json` `bundle.macOS.frameworks` reference; setup hook calls `stint_intents_init()`.
12. **Pull worker hook** — `pull_worker` calls `notify_indexer(ProjectsReplaced/TasksReplaced)` after each successful pull.
13. **CI gates** — `codesign --verify` step + `pluginkit -mvD` count assertion. Swift test step.
14. **Manual smoke** — checklist exercise on a release-mode `cargo tauri build` install.
15. **Docs** — extend `SKILL.md` with App Intents surface ladder, focus-filter race documentation, and stint:// URL route additions.

## 11. 6c outline (full spec deferred)

| Surface | Stack | Approx scope |
|---|---|---|
| Raycast extension | TypeScript, talks to `stint --json` subprocess | ~1.5 days |
| Alfred workflow | Bash/PHP scripts + Alfred workflow bundle | ~0.5 days |
| WidgetKit widget | Swift Widget Extension target in same SPM workspace | ~2 days |
| Idle detection | Rust `CGEventSourceSecondsSinceLastEventType` polling in `stint-app` + prompt UI | ~1 day |

6c consumes 6b's FFI bridge for the widget (which runs in its own process — would need a different IPC story; likely loopback HTTP since the widget process is short-lived and can't link the framework). Full spec written when 6b lands.

## 12. References

- Phase 6a façade: `crates/stint-core/src/verbs/mod.rs`
- Existing URL scheme parser: `crates/stint-core/src/url_scheme.rs`
- HTTP API handlers (same shapes): `crates/stint-app/src/http/handlers.rs`
- MCP server (same shapes): `crates/stint-cli/src/cmd/mcp.rs`
- SKILL.md (will be extended): `crates/stint-cli/skills/stint/SKILL.md`
- Tauri bundle config: `crates/stint-app/tauri.conf.json`
- Tauri entitlements: `crates/stint-app/entitlements.plist`

Apple references:
- App Intents framework — [`developer.apple.com/documentation/appintents`](https://developer.apple.com/documentation/appintents)
- Core Spotlight — [`developer.apple.com/documentation/corespotlight`](https://developer.apple.com/documentation/corespotlight)
- `SetFocusFilterIntent` — [`developer.apple.com/documentation/appintents/setfocusfilterintent`](https://developer.apple.com/documentation/appintents/setfocusfilterintent)
- App Shortcuts phrase guidelines — [`developer.apple.com/documentation/appintents/app-shortcuts`](https://developer.apple.com/documentation/appintents/app-shortcuts)
