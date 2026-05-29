# Stint Phase 6d — Xcode-Based Extensions Migration

**Status:** design
**Date:** 2026-05-28
**Predecessors:** Phase 6b (`docs/superpowers/specs/2026-05-25-stint-phase-6-deeper-integration-design.md` §1.5), Phase 6c (`docs/superpowers/specs/2026-05-27-stint-phase-6c-power-user-surfaces-design.md` §6)

---

## 1. Why this phase exists

Two surfaces shipped in 6b + 6c are **structurally complete but functionally inert** because Apple's extension runtime rejects them at bootstrap:

- **App Intents (Siri / Shortcuts.app / Focus filter UI)** — 6b shipped the Swift code in an embedded framework. Apple's intent indexer (`siriactionsd`) only discovers types declared in the main binary or in a real **App Intents Extension** `.appex` bundle. Framework-embedded intents are invisible to Siri and Shortcuts.app.

- **WidgetKit widget** — 6c shipped a Swift Package compiled to a `.appex`-shaped bundle. `pluginkit` registers it, `chronod` launches it, but the binary crashes immediately in Apple's private `_EXRunningExtension.sharedInstance()` with `Failed to create running extension of type: 'viewBridgeUI'`. The runtime needs metadata that Xcode's "Widget Extension" target template injects and SPM's `executableTarget` doesn't.

Both failures share a single root cause: **Apple's extension architecture has runtime-metadata requirements that only Xcode's extension-target templates produce.** This phase replaces the SPM-based Swift build with an Xcode-driven build, unblocking both surfaces in one migration.

Out of scope: no new user-facing features. This phase makes shipped-but-broken surfaces actually work.

---

## 2. Goals

- Siri voice ("Hey Siri, start tracking in Stint") works.
- Shortcuts.app discovers and lists stint's App Intents (Start, Stop, Current, List Today, Switch Project, etc.).
- System Settings → Focus → Stint shows the per-focus project picker UI.
- Right-click desktop → Edit Widgets → Stint appears with three configs × two sizes.
- Spotlight indexing continues to work (no regression from current framework path).

Non-goals:

- New App Intent types beyond what 6b already defined.
- New widget kinds beyond the three already designed.
- iOS / iPadOS support.
- Replacing `xcodebuild` with a Rust-native implementation. (Discussed in §11.3.)

---

## 3. Architecture

```
crates/stint-app/swift/
  xcodegen/
    project.yml                          # NEW — single source of truth
    .gitignore                           # ignores StintExtensions.xcodeproj/

  StintExtensionsCore/                   # NEW — shared framework target
    Sources/
      PortDiscovery.swift                # moved from StintWidget/Sources/StintWidget/Models/
      EntryDTO.swift                     # moved
      ProjectDTO.swift                   # moved
      SpotlightIndexer.swift             # moved from StintIntents/Sources/StintIntents/Spotlight/
      Entities/                          # moved from StintIntents/Sources/StintIntents/Entities/
      Intents/                           # moved from StintIntents/Sources/StintIntents/Intents/
      Focus/                             # moved from StintIntents/Sources/StintIntents/Focus/
      Bridge/
        RustFFI.swift                    # moved from StintIntents/Sources/StintIntents/Bridge.swift
      IPC/
        SharedContainerMarker.swift      # NEW — reads/writes reindex marker file
        DarwinNotification.swift         # NEW — host-extension wakeup signal
    Tests/                               # consolidates today's StintIntents/Tests + StintWidget/Tests

  Extensions/
    StintIntentsExtension/               # NEW — App Intents Extension .appex
      Info.plist                         # NSExtensionPointIdentifier = com.apple.appintents-extension
      Sources/
        IntentsExtensionMain.swift       # @main AppIntentsExtension { var body: ... }
        ExtensionLifecycle.swift         # observes Darwin notification, drains marker
      StintIntentsExtension.entitlements # sandbox + app-group

    StintWidget/                         # NEW — Widget Extension .appex
      Info.plist                         # NSExtensionPointIdentifier = com.apple.widgetkit-extension
      Sources/
        WidgetMain.swift                 # moved from StintWidget/Sources/StintWidget/StintWidgetBundle.swift
        RunningTimerWidget.swift         # moved
        Provider.swift                   # moved
        WidgetConfigIntent.swift         # moved
        Views/                           # moved
      StintWidget.entitlements           # sandbox + app-group + network.client

  StintIntents/                          # DELETED
  StintWidget/                           # DELETED
```

The repo loses two SPM package directories (`StintIntents/`, `StintWidget/`) and gains one declarative project file (`xcodegen/project.yml`) plus three Xcode build targets (`StintExtensionsCore`, `StintIntentsExtension`, `StintWidget`).

---

## 4. Build flow

```
build.rs (stint-app)
  ├─ check xcodegen is installed; on miss emit `cargo:warning=brew install xcodegen` and bail
  ├─ run `xcodegen generate` in swift/xcodegen/ → StintExtensions.xcodeproj
  ├─ run `xcodebuild build` for scheme StintIntentsExtension → .appex artifact
  ├─ run `xcodebuild build` for scheme StintWidget → .appex artifact
  ├─ copy both .appex bundles into crates/stint-app/PlugIns/
  ├─ ad-hoc codesign both bundles for local dev
  └─ (release path: scripts/build-app-with-widget.sh re-signs with Developer ID + entitlements)
```

`cargo:rerun-if-changed=` covers `project.yml` plus all `.swift` files under both extension source trees.

`STINT_SKIP_SWIFT_BUILD=1` continues to skip the entire Xcode path (useful for stint-core-only iteration).

---

## 5. IPC: host → extension wakeup for Spotlight reindex

The current dlsym path is synchronous and in-process. The extension path is asynchronous and eventually-consistent (within seconds). Acceptable for Spotlight — it's a search index, not a UI surface.

**Mechanism:**

- **App Group ID:** `group.tech.reyem.stint` — declared in both host and extension entitlements.
- **Shared container path:** `~/Library/Group Containers/group.tech.reyem.stint/`
- **Marker file:** `pending-reindex.json` — host writes atomically (write to temp file, rename); contains list of `{local_uuid, op}` entries where op ∈ `{insert, update, delete}`.
- **Darwin notification name:** `tech.reyem.stint.reindex` — host posts via `CFNotificationCenterPostNotification`. Extension registers an observer on launch (any launch — the indexer wakes the extension periodically anyway; the notification is best-effort eagerness).
- **Extension drain logic:** on launch + on notification, read marker file, perform Spotlight upserts/deletes per entry, then clear the file atomically.

**Rust side replacement:**

- Today `stint_app::commands::*` calls `dlsym(stint_notify_indexer)` after every entry mutation.
- New helper module `crates/stint-app/src/spotlight_ipc.rs`:
  - `push_pending(local_uuid: &str, op: SpotlightOp)` — appends to the marker file in the App Group container, posts the Darwin notification.
  - Replaces every existing `stint_notify_indexer` call.
- Drops `init_stint_intents()` and the framework dlsym scaffolding from `main.rs`.

**Recovery story:** if the extension never wakes (e.g. user disabled background activity for stint), the marker file accumulates. On the next wake, the extension drains the backlog — no data loss, only index staleness.

---

## 6. Migration order

Each step is independently verifiable. At no point are both the widget AND Spotlight broken simultaneously.

| Step | What lands | Verification |
|---|---|---|
| **A** | xcodegen `project.yml` + StintExtensionsCore framework target (with the widget-side shared types: PortDiscovery, EntryDTO, ProjectDTO) + StintWidget extension target. Legacy `swift/StintWidget/` package + `swift/StintIntents/` package still in tree, both unreferenced from the new build. | Widget appears in macOS gallery after install + notarize. |
| **B** | Add StintIntentsExtension target to `project.yml`. Move intent type declarations + Entities into StintExtensionsCore (extension target depends on it). Framework path (`swift/StintIntents/` SPM package) still actively building and serving Spotlight via dlsym. | Shortcuts.app discovers stint actions. Spotlight still works via the legacy framework. |
| **C** | Move SpotlightIndexer + Focus + RustFFI bridge into StintExtensionsCore. Add App Group entitlements to host + both extensions. Implement Darwin notification + marker file in `crates/stint-app/src/spotlight_ipc.rs`. Replace every existing `dlsym(stint_notify_indexer)` call with the new helper. | Spotlight indexing continues to work (mutate an entry, wait ~5s, search). |
| **D** | Delete `swift/StintIntents/` package. Delete `swift/StintWidget/` package. Remove framework build path from `build.rs`. Remove `bundle.macOS.frameworks` from `tauri.conf.json`. Remove `init_stint_intents()` + dlsym scaffolding from `main.rs`. | Full workspace test green; coverage script reports no regression. |

**Branching:** start from `main` after 6c lands. New branch `phase-6d`. Commits land via merge-commit PR to main, following the project ritual.

---

## 7. Tests

**Unit (xcodebuild test against StintExtensionsCore framework target):**

- Existing 19 StintIntents tests migrate verbatim — they test pure Swift types (DTO decoding, entity DTOs, etc.) that don't care which target hosts them.
- Existing 5 StintWidget tests (PortDiscovery, DTO coding) migrate verbatim.
- New: `SharedContainerMarkerTests` — write/read JSON atomically, list pending, clear, handle missing-file as empty.
- New: `DarwinNotificationTests` — register observer, post notification, observer fires within 1s (xctest with expectation).

**Rust integration:**

- `crates/stint-app/tests/spotlight_ipc.rs` — verify `push_pending()` writes to the expected App Group path, formats JSON correctly, posts the notification without panicking. Uses tempdir + an `STINT_APP_GROUP_DIR_OVERRIDE` env var to avoid touching the real container.

**Manual smoke (release-quality validation):**

1. Build + notarize + install to /Applications.
2. `pluginkit -m -p com.apple.widgetkit-extension | grep stint` → widget bundle ID listed.
3. `pluginkit -m -p com.apple.appintents-extension | grep stint` → intents bundle ID listed.
4. Right-click desktop → Edit Widgets → search "Stint" → expect three configs × small/medium.
5. Open Shortcuts.app → search "stint" → expect Start Timer / Stop / Current / List Today / etc.
6. Siri → "start tracking in Stint" → entry begins.
7. System Settings → Focus → pick a focus → Add Filter → expect Stint filter with project picker.
8. Spotlight test: start a timer with description "spec-test-X". Wait 5s. ⌘-Space → "spec-test-X" → entry result appears.

---

## 8. CI changes

**ci.yml:**

```yaml
- name: Install XcodeGen
  run: brew install xcodegen

- name: Generate Xcode project
  working-directory: crates/stint-app/swift/xcodegen
  run: xcodegen generate

- name: Swift test (StintExtensionsCore)
  run: xcodebuild test -scheme StintExtensionsCore \
       -destination 'platform=macOS' \
       -derivedDataPath ./build/derived
```

Removes the two separate `Swift test (StintIntents)` and `Swift test (StintWidget)` steps from today's ci.yml — they collapse into one against the shared framework.

**release-artifacts.yml:**

```yaml
- name: Install XcodeGen
  run: brew install xcodegen

# (xcodegen generate runs inside cargo build via build.rs, no separate step needed)

- name: Sign both .appex bundles with Developer ID + entitlements
  env:
    APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}
  run: |
    codesign --force --options runtime --timestamp \
      --sign "$APPLE_SIGNING_IDENTITY" \
      --entitlements crates/stint-app/swift/Extensions/StintIntentsExtension/StintIntentsExtension.entitlements \
      "$APP_PATH/Contents/PlugIns/StintIntentsExtension.appex"
    codesign --force --options runtime --timestamp \
      --sign "$APPLE_SIGNING_IDENTITY" \
      --entitlements crates/stint-app/swift/Extensions/StintWidget/StintWidget.entitlements \
      "$APP_PATH/Contents/PlugIns/StintWidget.appex"
    # main binary + bundle re-sign as today (with App Group entitlement added
    # to entitlements.plist)
```

Removes the framework-signing step and the standalone widget-signing step that 6c added. Replaces with the two-appex signing block above.

---

## 9. Local-dev impact

`scripts/dev-app.sh` adds a `command -v xcodegen` check up-front; prints `brew install xcodegen` and exits 1 if missing.

`README.md` first-time-setup section gains `brew install xcodegen` alongside `pnpm` and `cargo install tauri-cli`.

`STINT_SKIP_SWIFT_BUILD=1` continues to fully skip the Xcode path for non-Swift iterating.

---

## 10. Entitlements

**`crates/stint-app/entitlements.plist`** (host) gains:

```xml
<key>com.apple.security.application-groups</key>
<array>
    <string>group.tech.reyem.stint</string>
</array>
```

**`StintIntentsExtension.entitlements`:**

```xml
<key>com.apple.security.app-sandbox</key>
<true/>
<key>com.apple.security.application-groups</key>
<array>
    <string>group.tech.reyem.stint</string>
</array>
```

**`StintWidget.entitlements`** (already exists from 6c) adds:

```xml
<key>com.apple.security.application-groups</key>
<array>
    <string>group.tech.reyem.stint</string>
</array>
```

---

## 11. Trade-offs + open questions

### 11.1 XcodeGen as a build dependency

Adding a Homebrew dep for first-time setup. Mitigated by README + dev-script check. XcodeGen is widely used (1Password, Mozilla, Bitwarden) and stable.

### 11.2 Two `.appex` bundles instead of one framework

Slightly larger `Stint.app` (each `.appex` carries its own Swift runtime overhead, ~1-2 MB each). Acceptable for the functional gains.

### 11.3 Could we replace xcodebuild with a Rust library later?

The metadata Apple's extension runtime consults (`Metadata.appintents/extract.actionsdata` schema, `__TEXT,__appintents_meta` Mach-O section layout, `_EXRunningExtension` registration) is **undocumented and changes between macOS releases**. A Rust replacement would be perpetually catching up to Apple's private contract. Recommendation: don't. After 6d lands, we'll have concrete data about what's in the binaries — revisit only if that contract turns out to be small and stable.

### 11.4 What if a future macOS release breaks the contract?

Same exposure we already accept by using Apple's frameworks at all. Mitigation: subscribe to the macOS beta cycle (Apple ships beta SDKs in WWDC); recompile + test before the public release. The 6b framework path had the same risk; this phase doesn't change the exposure surface, only its shape.

### 11.5 What if XcodeGen project.yml expressiveness runs out?

If we hit a build setting xcodegen can't express, we can either pin to a specific Xcode-generated `.xcodeproj` for the affected target (committing the file as one-time), or switch to Tuist for that target. Treat as a future maintenance issue, not a blocker.

---

## 12. Success criteria

- All eight manual smoke tests in §7 pass on a notarized build installed to `/Applications/`.
- Existing test suites (workspace cargo, UI vitest, StintExtensionsCore xcodebuild test) all green.
- Coverage: `scripts/coverage.sh` reports no surface regression below 80%.
- Spotlight indexing of mutated entries observable within 10 seconds.
- The roadmap rows for 6b and 6c flip from "partial" / "shipped with caveat" to fully shipped; 6d ships as "deferred-scope-from-6b+6c resolved".

---

## 13. Out-of-scope reminders

- No new App Intent types.
- No new widget kinds, sizes, or configurations.
- No iOS / iPadOS port.
- No Rust-native xcodebuild replacement.
- No changes to Raycast extension, Alfred workflow, idle detection, or HTTP API.

If any of these emerge as worthwhile during execution, they belong in a separate follow-up phase.
