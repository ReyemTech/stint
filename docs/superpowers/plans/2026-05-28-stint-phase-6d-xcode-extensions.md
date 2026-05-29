# Phase 6d — Xcode-Based Extensions Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace stint-app's SPM-based Swift build with an xcodegen-driven Xcode project hosting one shared framework target plus two extension targets, unblocking both the deferred 6b App Intents work and the broken 6c WidgetKit loading.

**Architecture:** XcodeGen-generated `.xcodeproj` with three targets: `StintExtensionsCore` (shared framework), `StintIntentsExtension` (App Intents `.appex`), `StintWidget` (Widget `.appex`). `build.rs` runs `xcodegen generate` + `xcodebuild build` for each extension scheme, repackages the resulting `.appex` bundles into `crates/stint-app/PlugIns/` for Tauri to embed. Spotlight indexing moves from in-process (dlsym from stint-app into the framework) to cross-process (host writes pending-reindex marker to App Group container, posts Darwin notification, extension drains).

**Tech Stack:** XcodeGen (`brew install xcodegen`), xcodebuild, Swift 5.9+, macOS 14+ (App Intents Extensions require it), Core Foundation Darwin notifications, App Groups, Rust `libc::dlsym` (retained for cross-binary calls but pointed at new symbols).

**Branch:** Start from `main` AFTER the 6c PR lands. New branch `phase-6d`. Do NOT execute this plan from `feature/task-assignment` — the spec's Step D deletes paths that 6c depends on, and the diff against main will be unreadable if 6c hasn't merged first.

**Pre-flight:**

```bash
# Ensure 6c is on main first
git checkout main && git pull
git log --oneline | head -5  # expect 6c commits at top

# Required toolchain
brew install xcodegen       # ~5 MB; pure Swift
xcodebuild -version         # expect Xcode 15+
swift --version             # expect Swift 5.9+

# Start the branch
git checkout -b phase-6d
```

**Phase exit criteria** (each independently shippable per spec §6):

- **Phase A** ✅ when the WidgetKit widget appears in the macOS Edit Widgets gallery after notarized install.
- **Phase B** ✅ when `pluginkit -m -p com.apple.appintents-extension | grep stint` lists the intents extension AND Shortcuts.app discovers stint actions.
- **Phase C** ✅ when mutating an entry triggers Spotlight reindex within ~10 seconds (test: change description, wait, search for new text).
- **Phase D** ✅ when the legacy framework path is gone, full `scripts/coverage.sh` is green, and all 8 manual smoke checks from spec §7 pass.

**NEVER push or merge to main without explicit user approval. NEVER trigger releases. NEVER use `--no-verify` or `--no-gpg-sign` unless the user explicitly asks for it.**

---

# Phase A — XcodeGen + Widget Extension

Goal: replace the SPM-based widget build with an xcodegen-driven Xcode build of one shared framework + one Widget Extension target. End state: the widget appears in the macOS Edit Widgets gallery.

---

## Task A1: Document the xcodegen dependency

**Files:**
- Modify: `scripts/dev-app.sh`
- Modify: `README.md`
- Modify: `CLAUDE.md`

- [ ] **Step 1: Add xcodegen check to scripts/dev-app.sh**

Find the line near the top that does dependency checks (look for `command -v` or the comment block about first-time setup). Add this guard just before the first `cargo build` invocation:

```bash
# Phase 6d: xcodegen drives the Swift extension builds.
if ! command -v xcodegen >/dev/null 2>&1; then
  echo "error: xcodegen not installed. Install: brew install xcodegen" >&2
  exit 1
fi
```

- [ ] **Step 2: Update README.md first-time setup**

Find the "First-time setup on a fresh machine" section in README.md. Update the brew install line to include xcodegen:

```bash
brew install pnpm rust xcodegen
```

- [ ] **Step 3: Update CLAUDE.md Gotchas section**

Append a new bullet to the "Gotchas / dev-environment notes" section of `CLAUDE.md`:

```markdown
- **xcodegen drives Swift extension builds.** As of Phase 6d, the `.xcodeproj`
  that produces `StintIntentsExtension.appex` and `StintWidget.appex` is
  generated from `crates/stint-app/swift/xcodegen/project.yml` by xcodegen at
  build time. The `.xcodeproj` itself is gitignored — never commit it.
  Install once: `brew install xcodegen`. `scripts/dev-app.sh` checks for it
  and fails fast with a clear error if missing.
```

- [ ] **Step 4: Commit**

```bash
git add scripts/dev-app.sh README.md CLAUDE.md
git commit -m "docs(6d): xcodegen dependency for Swift extension builds"
```

---

## Task A2: Scaffold xcodegen directory + .gitignore

**Files:**
- Create: `crates/stint-app/swift/xcodegen/.gitignore`
- Create: `crates/stint-app/swift/xcodegen/README.md`

- [ ] **Step 1: Create the gitignore**

```bash
mkdir -p crates/stint-app/swift/xcodegen
cat > crates/stint-app/swift/xcodegen/.gitignore <<'EOF'
StintExtensions.xcodeproj/
build/
.build/
DerivedData/
EOF
```

- [ ] **Step 2: Create the README**

```bash
cat > crates/stint-app/swift/xcodegen/README.md <<'EOF'
# StintExtensions Xcode project source

The `.xcodeproj` here is generated from `project.yml` by [xcodegen](https://github.com/yonaskolb/XcodeGen). Never edit `StintExtensions.xcodeproj/` directly — it's gitignored and regenerated on every build.

## Manual regenerate (rare)

```bash
cd crates/stint-app/swift/xcodegen
xcodegen generate
open StintExtensions.xcodeproj   # if you want to inspect in Xcode
```

`build.rs` runs `xcodegen generate` automatically before each `xcodebuild` invocation.

## Targets

- `StintExtensionsCore` — framework: shared Swift code (DTOs, PortDiscovery, IPC helpers, intent type declarations, Spotlight indexer).
- `StintIntentsExtension` — App Intents Extension `.appex`: registers intents with Siri/Shortcuts/Focus, drains Spotlight reindex queue.
- `StintWidget` — Widget Extension `.appex`: WidgetKit widget bundle.
- `StintExtensionsCoreTests` — test target against `StintExtensionsCore`.
EOF
```

- [ ] **Step 3: Commit**

```bash
git add crates/stint-app/swift/xcodegen/
git commit -m "chore(6d): scaffold xcodegen/ directory"
```

---

## Task A3: Create StintExtensionsCore source skeleton + copy widget-side DTOs

**Files:**
- Create: `crates/stint-app/swift/StintExtensionsCore/Sources/Models/PortDiscovery.swift` (copy from legacy)
- Create: `crates/stint-app/swift/StintExtensionsCore/Sources/Models/EntryDTO.swift` (copy)
- Create: `crates/stint-app/swift/StintExtensionsCore/Sources/Models/ProjectDTO.swift` (copy)

The legacy `crates/stint-app/swift/StintWidget/Sources/StintWidget/Models/` directory has the originals. Copy them — the legacy SPM package stays in tree until Step D, so we duplicate rather than move.

- [ ] **Step 1: Create the directory tree**

```bash
mkdir -p crates/stint-app/swift/StintExtensionsCore/Sources/Models
mkdir -p crates/stint-app/swift/StintExtensionsCore/Tests
```

- [ ] **Step 2: Copy the three model files**

```bash
cp crates/stint-app/swift/StintWidget/Sources/StintWidget/Models/PortDiscovery.swift \
   crates/stint-app/swift/StintExtensionsCore/Sources/Models/PortDiscovery.swift
cp crates/stint-app/swift/StintWidget/Sources/StintWidget/Models/EntryDTO.swift \
   crates/stint-app/swift/StintExtensionsCore/Sources/Models/EntryDTO.swift
cp crates/stint-app/swift/StintWidget/Sources/StintWidget/Models/ProjectDTO.swift \
   crates/stint-app/swift/StintExtensionsCore/Sources/Models/ProjectDTO.swift
```

- [ ] **Step 3: Verify files exist and are non-empty**

```bash
wc -l crates/stint-app/swift/StintExtensionsCore/Sources/Models/*.swift
```

Expected: three files, each between 10 and 50 lines.

- [ ] **Step 4: Commit**

```bash
git add crates/stint-app/swift/StintExtensionsCore/
git commit -m "chore(6d): copy widget DTO + PortDiscovery into StintExtensionsCore"
```

---

## Task A4: Migrate widget test files into StintExtensionsCoreTests

**Files:**
- Create: `crates/stint-app/swift/StintExtensionsCore/Tests/PortDiscoveryTests.swift` (copy from legacy)
- Create: `crates/stint-app/swift/StintExtensionsCore/Tests/DTOCodingTests.swift` (copy from legacy)

- [ ] **Step 1: Copy both test files**

```bash
cp crates/stint-app/swift/StintWidget/Tests/StintWidgetTests/PortDiscoveryTests.swift \
   crates/stint-app/swift/StintExtensionsCore/Tests/PortDiscoveryTests.swift
cp crates/stint-app/swift/StintWidget/Tests/StintWidgetTests/DTOCodingTests.swift \
   crates/stint-app/swift/StintExtensionsCore/Tests/DTOCodingTests.swift
```

- [ ] **Step 2: Update the `@testable import` lines**

Both files have `@testable import StintWidget` on a line near the top. Change to `@testable import StintExtensionsCore` in both files:

```bash
sed -i '' 's/@testable import StintWidget$/@testable import StintExtensionsCore/' \
  crates/stint-app/swift/StintExtensionsCore/Tests/PortDiscoveryTests.swift \
  crates/stint-app/swift/StintExtensionsCore/Tests/DTOCodingTests.swift
```

- [ ] **Step 3: Verify the import line**

```bash
grep "import StintExtensionsCore" crates/stint-app/swift/StintExtensionsCore/Tests/*.swift
```

Expected: one match per file (both files).

- [ ] **Step 4: Commit**

```bash
git add crates/stint-app/swift/StintExtensionsCore/Tests/
git commit -m "test(6d): migrate widget DTO + PortDiscovery tests to StintExtensionsCoreTests"
```

---

## Task A5: Create project.yml with StintExtensionsCore framework target only

**Files:**
- Create: `crates/stint-app/swift/xcodegen/project.yml`

- [ ] **Step 1: Write project.yml with just the framework + test targets**

This first version has no extension targets yet — we verify the framework builds + tests pass in isolation before adding extensions.

```yaml
name: StintExtensions

options:
  deploymentTarget:
    macOS: "14.0"
  bundleIdPrefix: tech.reyem.stint
  createIntermediateGroups: true
  developmentLanguage: en

settings:
  base:
    SWIFT_VERSION: "5.9"
    MACOSX_DEPLOYMENT_TARGET: "14.0"
    ENABLE_HARDENED_RUNTIME: YES
    CODE_SIGN_STYLE: Manual
    CODE_SIGN_IDENTITY: "-"

targets:
  StintExtensionsCore:
    type: framework
    platform: macOS
    sources:
      - path: ../StintExtensionsCore/Sources
    info:
      path: ../StintExtensionsCore/Info.plist
      properties:
        CFBundleIdentifier: tech.reyem.stint.extensions.core
    settings:
      base:
        PRODUCT_NAME: StintExtensionsCore
        PRODUCT_BUNDLE_IDENTIFIER: tech.reyem.stint.extensions.core
        DEFINES_MODULE: YES
        SKIP_INSTALL: NO

  StintExtensionsCoreTests:
    type: bundle.unit-test
    platform: macOS
    sources:
      - path: ../StintExtensionsCore/Tests
    dependencies:
      - target: StintExtensionsCore
    settings:
      base:
        BUNDLE_LOADER: "$(TEST_HOST)"
        PRODUCT_BUNDLE_IDENTIFIER: tech.reyem.stint.extensions.core.tests
```

- [ ] **Step 2: Generate the project and verify**

```bash
cd crates/stint-app/swift/xcodegen
xcodegen generate 2>&1 | tail -5
ls -d StintExtensions.xcodeproj
cd -
```

Expected: `Loaded project ... Generated project successfully.` and the `.xcodeproj` directory exists.

- [ ] **Step 3: Run the test suite to verify the framework + tests compile + pass**

```bash
cd crates/stint-app/swift/xcodegen
xcodebuild test -scheme StintExtensionsCoreTests -destination 'platform=macOS' -derivedDataPath ./build/derived 2>&1 | tail -8
cd -
```

Expected: `** TEST SUCCEEDED **` and `5 tests` reported.

- [ ] **Step 4: Commit**

```bash
git add crates/stint-app/swift/xcodegen/project.yml
git commit -m "feat(6d): xcodegen project.yml — StintExtensionsCore framework + tests"
```

---

## Task A6: Copy widget source into Extensions/StintWidget/

**Files:**
- Create: `crates/stint-app/swift/Extensions/StintWidget/Sources/WidgetMain.swift` (copy of StintWidgetBundle.swift)
- Create: `crates/stint-app/swift/Extensions/StintWidget/Sources/RunningTimerWidget.swift` (copy)
- Create: `crates/stint-app/swift/Extensions/StintWidget/Sources/Provider.swift` (copy)
- Create: `crates/stint-app/swift/Extensions/StintWidget/Sources/WidgetConfigIntent.swift` (copy)
- Create: `crates/stint-app/swift/Extensions/StintWidget/Sources/Views/RunningTimerView.swift` (copy)
- Create: `crates/stint-app/swift/Extensions/StintWidget/Sources/Views/TodayTotalView.swift` (copy)
- Create: `crates/stint-app/swift/Extensions/StintWidget/Sources/Views/WeekProjectView.swift` (copy)

- [ ] **Step 1: Create the directory tree**

```bash
mkdir -p crates/stint-app/swift/Extensions/StintWidget/Sources/Views
```

- [ ] **Step 2: Copy the files**

```bash
cp crates/stint-app/swift/StintWidget/Sources/StintWidget/StintWidgetBundle.swift \
   crates/stint-app/swift/Extensions/StintWidget/Sources/WidgetMain.swift
cp crates/stint-app/swift/StintWidget/Sources/StintWidget/RunningTimerWidget.swift \
   crates/stint-app/swift/Extensions/StintWidget/Sources/
cp crates/stint-app/swift/StintWidget/Sources/StintWidget/Provider.swift \
   crates/stint-app/swift/Extensions/StintWidget/Sources/
cp crates/stint-app/swift/StintWidget/Sources/StintWidget/WidgetConfigIntent.swift \
   crates/stint-app/swift/Extensions/StintWidget/Sources/
cp crates/stint-app/swift/StintWidget/Sources/StintWidget/Views/*.swift \
   crates/stint-app/swift/Extensions/StintWidget/Sources/Views/
```

- [ ] **Step 3: Add `import StintExtensionsCore` to files that reference moved types**

`Provider.swift` references `PortDiscovery`, `EntryDTO`. `WidgetConfigIntent.swift` references `PortDiscovery`, `ProjectDTO`. Add the import after the existing `import Foundation` / `import WidgetKit` lines.

```bash
for f in \
  crates/stint-app/swift/Extensions/StintWidget/Sources/Provider.swift \
  crates/stint-app/swift/Extensions/StintWidget/Sources/WidgetConfigIntent.swift; do
  # Insert "import StintExtensionsCore" after the last existing "import " line
  awk '/^import / { last=NR } { lines[NR]=$0 } END {
    for (i=1; i<=NR; i++) { print lines[i]; if (i==last) print "import StintExtensionsCore" }
  }' "$f" > "$f.tmp" && mv "$f.tmp" "$f"
done
```

- [ ] **Step 4: Verify the imports**

```bash
grep -l "import StintExtensionsCore" crates/stint-app/swift/Extensions/StintWidget/Sources/{Provider,WidgetConfigIntent}.swift
```

Expected: both files listed.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-app/swift/Extensions/StintWidget/Sources/
git commit -m "chore(6d): copy widget source into Extensions/StintWidget/"
```

---

## Task A7: Create Info.plist + entitlements for the widget extension target

**Files:**
- Create: `crates/stint-app/swift/Extensions/StintWidget/Info.plist`
- Create: `crates/stint-app/swift/Extensions/StintWidget/StintWidget.entitlements`

- [ ] **Step 1: Write the Info.plist**

This is the same shape that worked end-of-6c (after the fix in commit `52bb6a4`): all platform-identification keys present.

```bash
cat > crates/stint-app/swift/Extensions/StintWidget/Info.plist <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>tech.reyem.stint.widget</string>
    <key>CFBundleExecutable</key>
    <string>StintWidget</string>
    <key>CFBundleName</key>
    <string>StintWidget</string>
    <key>CFBundleDisplayName</key>
    <string>Stint Widget</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundlePackageType</key>
    <string>XPC!</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleSupportedPlatforms</key>
    <array>
        <string>MacOSX</string>
    </array>
    <key>DTPlatformName</key>
    <string>macosx</string>
    <key>LSMinimumSystemVersion</key>
    <string>14.0</string>
    <key>NSExtension</key>
    <dict>
        <key>NSExtensionPointIdentifier</key>
        <string>com.apple.widgetkit-extension</string>
    </dict>
</dict>
</plist>
EOF
```

- [ ] **Step 2: Write the entitlements**

```bash
cat > crates/stint-app/swift/Extensions/StintWidget/StintWidget.entitlements <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.app-sandbox</key>
    <true/>
    <key>com.apple.security.network.client</key>
    <true/>
</dict>
</plist>
EOF
```

Note: the App Group entitlement is intentionally absent at this step — it gets added in Phase C when we wire IPC. Phase A's widget just fetches via HTTP loopback.

- [ ] **Step 3: Commit**

```bash
git add crates/stint-app/swift/Extensions/StintWidget/Info.plist \
        crates/stint-app/swift/Extensions/StintWidget/StintWidget.entitlements
git commit -m "feat(6d): Info.plist + entitlements for new StintWidget extension target"
```

---

## Task A8: Add StintWidget extension target to project.yml

**Files:**
- Modify: `crates/stint-app/swift/xcodegen/project.yml`

- [ ] **Step 1: Append the StintWidget target**

Open `crates/stint-app/swift/xcodegen/project.yml`. Below the existing `StintExtensionsCoreTests:` block (which is the last target), add:

```yaml
  StintWidget:
    type: app-extension
    platform: macOS
    sources:
      - path: ../Extensions/StintWidget/Sources
    info:
      path: ../Extensions/StintWidget/Info.plist
    entitlements:
      path: ../Extensions/StintWidget/StintWidget.entitlements
    dependencies:
      - target: StintExtensionsCore
        embed: false
    settings:
      base:
        PRODUCT_NAME: StintWidget
        PRODUCT_BUNDLE_IDENTIFIER: tech.reyem.stint.widget
        WRAPPER_EXTENSION: appex
        SKIP_INSTALL: YES
        LD_RUNPATH_SEARCH_PATHS:
          - "@executable_path/../../Frameworks"
        SWIFT_OPTIMIZATION_LEVEL: "-Onone"
    info:
      path: ../Extensions/StintWidget/Info.plist
      properties:
        NSExtension:
          NSExtensionPointIdentifier: com.apple.widgetkit-extension
```

- [ ] **Step 2: Regenerate + build the new target**

```bash
cd crates/stint-app/swift/xcodegen
xcodegen generate 2>&1 | tail -3
xcodebuild build -scheme StintWidget -configuration Release -destination 'platform=macOS' -derivedDataPath ./build/derived 2>&1 | tail -5
cd -
```

Expected: `** BUILD SUCCEEDED **`.

- [ ] **Step 3: Verify the produced .appex shape**

```bash
APPEX="crates/stint-app/swift/xcodegen/build/derived/Build/Products/Release/StintWidget.appex"
ls "$APPEX/Contents/" && file "$APPEX/Contents/MacOS/StintWidget"
```

Expected: `Info.plist`, `MacOS/`, `Frameworks/` directories. The Mach-O is `Mach-O 64-bit executable arm64` (or universal).

- [ ] **Step 4: Commit**

```bash
git add crates/stint-app/swift/xcodegen/project.yml
git commit -m "feat(6d): xcodegen StintWidget extension target — produces real .appex"
```

---

## Task A9: Swap build.rs to drive xcodegen + xcodebuild for the widget

**Files:**
- Modify: `crates/stint-app/build.rs`

The current `build_stint_widget()` function calls `xcodebuild` against the legacy SPM Package.swift. Replace its body with one that runs `xcodegen generate` + `xcodebuild build` against the new project.yml. The legacy SPM widget package stays in tree (Step D deletes it) but is no longer consumed by build.rs.

- [ ] **Step 1: Replace build_stint_widget() entirely**

Open `crates/stint-app/build.rs`. Replace the entire `fn build_stint_widget()` function (and its doc comment) with this:

```rust
/// Build the StintWidget app extension via xcodegen + xcodebuild and
/// place the resulting `.appex` bundle at
/// `crates/stint-app/PlugIns/StintWidget.appex/` where Tauri's bundle
/// step picks it up. Set `STINT_SKIP_SWIFT_BUILD=1` to skip.
fn build_stint_widget() -> Result<(), String> {
    if env::var_os("STINT_SKIP_SWIFT_BUILD").is_some_and(|v| !v.is_empty()) {
        return Err("STINT_SKIP_SWIFT_BUILD is set".into());
    }
    if env::var_os("CARGO_CFG_TARGET_OS").is_some_and(|v| v != "macos") {
        return Err("non-macOS target".into());
    }

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").map_err(|e| e.to_string())?;
    let xcodegen_dir = Path::new(&manifest_dir).join("swift/xcodegen");
    let project_yml = xcodegen_dir.join("project.yml");
    if !project_yml.exists() {
        return Err(format!("missing {}", project_yml.display()));
    }

    println!("cargo:rerun-if-changed={}", project_yml.display());
    let extensions_dir = Path::new(&manifest_dir).join("swift/Extensions/StintWidget");
    let core_dir = Path::new(&manifest_dir).join("swift/StintExtensionsCore");
    for src in [extensions_dir.as_path(), core_dir.as_path()] {
        if let Ok(entries) = fs::read_dir(src) {
            for entry in entries.flatten() {
                print_rerun_if_changed_recursive(&entry.path());
            }
        }
    }

    // Generate the .xcodeproj from project.yml (idempotent).
    let xcgen = Command::new("xcodegen")
        .current_dir(&xcodegen_dir)
        .arg("generate")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| format!("xcodegen spawn (is `brew install xcodegen` done?): {e}"))?;
    if !xcgen.success() {
        return Err(format!("xcodegen exit {xcgen}"));
    }

    let derived_data = xcodegen_dir.join("build/derived");
    let status = Command::new("xcodebuild")
        .current_dir(&xcodegen_dir)
        .args([
            "-scheme",
            "StintWidget",
            "-configuration",
            "Release",
            "-destination",
            "platform=macOS",
            "-derivedDataPath",
            derived_data.to_str().ok_or("derived path not utf8")?,
            "build",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| format!("xcodebuild spawn: {e}"))?;
    if !status.success() {
        return Err(format!("xcodebuild exit {status}"));
    }

    let built = derived_data.join("Build/Products/Release/StintWidget.appex");
    if !built.exists() {
        return Err(format!("missing {}", built.display()));
    }

    let dest = Path::new(&manifest_dir).join("PlugIns/StintWidget.appex");
    let _ = fs::remove_dir_all(&dest);
    fs::create_dir_all(dest.parent().unwrap()).map_err(|e| format!("create PlugIns/: {e}"))?;
    copy_dir(&built, &dest).map_err(|e| format!("copy appex: {e}"))?;

    codesign_adhoc(&dest).map_err(|e| format!("codesign appex: {e}"))?;

    println!(
        "cargo:warning=StintWidget.appex rebuilt at {}",
        dest.display()
    );
    Ok(())
}
```

- [ ] **Step 2: Cargo build to verify**

```bash
cargo build -p stint-app 2>&1 | tail -6
```

Expected: `Finished` line plus a `cargo:warning=StintWidget.appex rebuilt at …` line.

- [ ] **Step 3: Verify the produced bundle**

```bash
file crates/stint-app/PlugIns/StintWidget.appex/Contents/MacOS/StintWidget
ls crates/stint-app/PlugIns/StintWidget.appex/Contents/
```

Expected: `Mach-O 64-bit executable arm64` and directories `Frameworks/ Info.plist MacOS/ _CodeSignature/`.

- [ ] **Step 4: Commit**

```bash
git add crates/stint-app/build.rs
git commit -m "build(6d): build.rs drives xcodegen + xcodebuild for StintWidget appex"
```

---

## Task A10: Update build-app-with-widget.sh for new bundle layout

**Files:**
- Modify: `scripts/build-app-with-widget.sh`

The script's logic is correct; only thing changing is the `.appex` now ships a `Frameworks/StintExtensionsCore.framework` inside it (from the framework dep). The existing relocation + sign + verify still works as-is. Verify with a dry run.

- [ ] **Step 1: Run the wrapper script with ad-hoc sign**

```bash
scripts/build-app-with-widget.sh 2>&1 | tail -10
```

Expected: `Done. Bundle at target/release/bundle/macos/Stint.app` and a successful `codesign --verify`.

- [ ] **Step 2: Verify the appex contains the embedded framework**

```bash
ls target/release/bundle/macos/Stint.app/Contents/PlugIns/StintWidget.appex/Contents/Frameworks/
```

Expected: `StintExtensionsCore.framework`.

- [ ] **Step 3: Commit** (no source change, marker only)

```bash
git commit --allow-empty -m "test(6d): verified xcodegen-built widget appex bundles cleanly via wrapper"
```

---

## Task A11: Notarize, install, and verify widget gallery

**Files:** none.

This is a manual verification gate. The output of this task is a paste of `pluginkit -m -p com.apple.widgetkit-extension | grep stint` into the commit message.

- [ ] **Step 1: Sign with Developer ID + notarize**

```bash
scripts/build-app-with-widget.sh "Developer ID Application: Reyem Technologies Inc. (WAK5K2758P)"

APP="target/release/bundle/macos/Stint.app"
ZIP="${APP}.zip"
rm -f "$ZIP"
ditto -c -k --keepParent "$APP" "$ZIP"

xcrun notarytool submit "$ZIP" --keychain-profile "stint-notary" --wait
```

Expected: `status: Accepted`.

- [ ] **Step 2: Staple + install**

```bash
xcrun stapler staple "$APP"
killall stint-app 2>/dev/null; sleep 1
rm -rf /Applications/Stint.app
cp -R "$APP" /Applications/
xattr -cr /Applications/Stint.app
/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister -f /Applications/Stint.app
open /Applications/Stint.app
sleep 6
```

- [ ] **Step 3: Verify pluginkit registers the widget**

```bash
pluginkit -m -p com.apple.widgetkit-extension | grep -i stint
```

Expected: `tech.reyem.stint.widget(1.0)` appears.

- [ ] **Step 4: Manually verify the gallery**

1. Right-click the desktop → Edit Widgets.
2. Search "Stint".
3. Expect the Stint widget tile with three configurations (Running Timer / Today Total / This-Week Project) × two sizes (small, medium).
4. Drag one onto the desktop.
5. The widget renders the "Stint not running" placeholder (HTTP API isn't auto-enabled until Phase C wires the new IPC), OR — if you previously enabled `api.enabled = true` in Settings — it shows the current timer.

- [ ] **Step 5: Commit verification marker**

```bash
git commit --allow-empty -m "test(6d): Phase A — widget appears in macOS Edit Widgets gallery

pluginkit confirms tech.reyem.stint.widget(1.0) registered after
notarized install. Gallery shows configurable widget with both sizes.
End-state of Phase A reached: SPM widget build replaced with xcodegen-
driven build; widget loads + renders without the EXRunningExtension
crash that 6c hit."
```

---

## Task A12: Add xcodegen + StintExtensionsCore test step to CI

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Add xcodegen install step before the Swift test steps**

Open `.github/workflows/ci.yml`. Find the existing line `- name: Swift test (StintIntents framework)` (around line 57). Add a new step ABOVE it:

```yaml
      - name: Install XcodeGen
        run: brew install xcodegen

      - name: Generate Xcode project
        working-directory: crates/stint-app/swift/xcodegen
        run: xcodegen generate

      - name: Swift test (StintExtensionsCore)
        working-directory: crates/stint-app/swift/xcodegen
        run: xcodebuild test -scheme StintExtensionsCoreTests -destination 'platform=macOS' -derivedDataPath ./build/derived
```

Keep the existing `Swift test (StintIntents framework)` and `Swift test (StintWidget)` steps in place — they cover the legacy SPM packages, which Phase D deletes. Phase A is additive.

- [ ] **Step 2: Verify yaml is valid**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))" && echo "ci.yml: valid YAML"
```

Expected: `ci.yml: valid YAML`.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci(6d): xcodegen install + StintExtensionsCore test step"
```

---

# Phase B — App Intents Extension target

Goal: introduce a real `.appex` for App Intents alongside the working framework, so Siri/Shortcuts/Focus start discovering the intent types. The legacy framework keeps running and serving Spotlight via dlsym throughout this phase.

---

## Task B1: Copy intent type sources into StintExtensionsCore

**Files:**
- Create: `crates/stint-app/swift/StintExtensionsCore/Sources/Entities/EntryEntity.swift` (copy)
- Create: `.../Entities/EntryQuery.swift` (copy)
- Create: `.../Entities/ProjectEntity.swift` (copy)
- Create: `.../Entities/ProjectQuery.swift` (copy)
- Create: `.../Entities/TaskEntity.swift` (copy)
- Create: `.../Entities/TaskQuery.swift` (copy)
- Create: `.../Errors/BridgeError.swift` (copy)
- Create: `.../Intents/StartTimerIntent.swift` (copy)
- Create: `.../Intents/StopTimerIntent.swift` (copy)
- Create: `.../Intents/GetCurrentIntent.swift` (copy)
- Create: `.../Intents/ListEntriesIntent.swift` (copy)
- Create: `.../Intents/ListProjectsIntent.swift` (copy)
- Create: `.../Intents/ListTasksIntent.swift` (copy)
- Create: `.../Intents/SwitchProjectIntent.swift` (copy)
- Create: `.../Intents/UpdateEntryIntent.swift` (copy)
- Create: `.../Intents/DeleteEntryIntent.swift` (copy)
- Create: `.../Intents/LogPastIntent.swift` (copy)
- Create: `.../Shortcuts/StintAppShortcutsProvider.swift` (copy)
- Create: `.../Shortcuts/PhraseStrings.xcstrings` (copy)
- Create: `.../Bridge/RustFFI.swift` (copy of Bridge.swift, renamed)

These copies leave the legacy framework's originals intact. Phase D deletes them.

- [ ] **Step 1: Create directory tree**

```bash
mkdir -p crates/stint-app/swift/StintExtensionsCore/Sources/{Entities,Errors,Intents,Shortcuts,Bridge}
```

- [ ] **Step 2: Bulk copy**

```bash
SRC=crates/stint-app/swift/StintIntents/Sources/StintIntents
DST=crates/stint-app/swift/StintExtensionsCore/Sources

cp $SRC/Entities/*.swift $DST/Entities/
cp $SRC/Errors/*.swift $DST/Errors/
cp $SRC/Intents/*.swift $DST/Intents/
cp $SRC/Shortcuts/StintAppShortcutsProvider.swift $DST/Shortcuts/
cp $SRC/Shortcuts/PhraseStrings.xcstrings $DST/Shortcuts/
cp $SRC/Bridge.swift $DST/Bridge/RustFFI.swift
```

- [ ] **Step 3: Sanity-count**

```bash
find crates/stint-app/swift/StintExtensionsCore/Sources -name "*.swift" | wc -l
```

Expected: `21` (3 Models + 6 Entities + 1 Error + 10 Intents + 1 Shortcuts + 1 Bridge — adjust if your inventory differs).

- [ ] **Step 4: Regenerate the project and verify the framework still compiles**

```bash
cd crates/stint-app/swift/xcodegen
xcodegen generate 2>&1 | tail -3
xcodebuild build -scheme StintExtensionsCore -configuration Release -destination 'platform=macOS' -derivedDataPath ./build/derived 2>&1 | tail -5
cd -
```

Expected: `** BUILD SUCCEEDED **`.

If there are compile errors about missing dependencies (e.g. `swift_indexer_notify` symbol or `init_swift_init` symbol), DON'T fix them by moving more code — instead, add `#if canImport(WidgetKit)` guards or `@available(macOS 14, *)` annotations to the offending types ONLY if the error is platform-related. For missing C symbols (Rust FFI), the bridge file expects those symbols to be available at link time; this is fine because the framework is built with `-undefined dynamic_lookup`.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-app/swift/StintExtensionsCore/Sources/
git commit -m "chore(6d): copy intent types + entities into StintExtensionsCore"
```

---

## Task B2: Create the StintIntentsExtension Info.plist + entitlements

**Files:**
- Create: `crates/stint-app/swift/Extensions/StintIntentsExtension/Info.plist`
- Create: `crates/stint-app/swift/Extensions/StintIntentsExtension/StintIntentsExtension.entitlements`

- [ ] **Step 1: Info.plist**

```bash
mkdir -p crates/stint-app/swift/Extensions/StintIntentsExtension
cat > crates/stint-app/swift/Extensions/StintIntentsExtension/Info.plist <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>tech.reyem.stint.intents</string>
    <key>CFBundleExecutable</key>
    <string>StintIntentsExtension</string>
    <key>CFBundleName</key>
    <string>StintIntentsExtension</string>
    <key>CFBundleDisplayName</key>
    <string>Stint Intents</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundlePackageType</key>
    <string>XPC!</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleSupportedPlatforms</key>
    <array>
        <string>MacOSX</string>
    </array>
    <key>DTPlatformName</key>
    <string>macosx</string>
    <key>LSMinimumSystemVersion</key>
    <string>14.0</string>
    <key>NSAppIntentsPackage</key>
    <true/>
    <key>EXAppExtensionAttributes</key>
    <dict>
        <key>EXExtensionPointIdentifier</key>
        <string>com.apple.appintents-extension</string>
    </dict>
</dict>
</plist>
EOF
```

- [ ] **Step 2: Entitlements**

```bash
cat > crates/stint-app/swift/Extensions/StintIntentsExtension/StintIntentsExtension.entitlements <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.app-sandbox</key>
    <true/>
    <key>com.apple.security.network.client</key>
    <true/>
</dict>
</plist>
EOF
```

App Group entitlement gets added in Phase C when Spotlight IPC lands.

- [ ] **Step 3: Commit**

```bash
git add crates/stint-app/swift/Extensions/StintIntentsExtension/
git commit -m "feat(6d): Info.plist + entitlements for StintIntentsExtension"
```

---

## Task B3: Write IntentsExtensionMain.swift (@main AppIntentsExtension)

**Files:**
- Create: `crates/stint-app/swift/Extensions/StintIntentsExtension/Sources/IntentsExtensionMain.swift`

- [ ] **Step 1: Create the source file**

```bash
mkdir -p crates/stint-app/swift/Extensions/StintIntentsExtension/Sources
cat > crates/stint-app/swift/Extensions/StintIntentsExtension/Sources/IntentsExtensionMain.swift <<'EOF'
import AppIntents
import StintExtensionsCore

@main
struct StintAppIntentsExtension: AppIntentsExtension {
    // The extension's app intents come from the StintExtensionsCore framework
    // via Apple's automatic discovery of any `AppIntent`-conforming type in
    // any linked module. No manual registration is required.
    //
    // Apple's intent indexer (siriactionsd) scans this binary's
    // Metadata.appintents stencil at install time and registers the discovered
    // intents with Siri, Shortcuts.app, and Focus filter UI.
}
EOF
```

- [ ] **Step 2: Commit**

```bash
git add crates/stint-app/swift/Extensions/StintIntentsExtension/Sources/
git commit -m "feat(6d): @main AppIntentsExtension entry point"
```

---

## Task B4: Add StintIntentsExtension target to project.yml

**Files:**
- Modify: `crates/stint-app/swift/xcodegen/project.yml`

- [ ] **Step 1: Append the new target**

Append to the bottom of `project.yml` (after the StintWidget target):

```yaml
  StintIntentsExtension:
    type: app-extension
    platform: macOS
    sources:
      - path: ../Extensions/StintIntentsExtension/Sources
    info:
      path: ../Extensions/StintIntentsExtension/Info.plist
    entitlements:
      path: ../Extensions/StintIntentsExtension/StintIntentsExtension.entitlements
    dependencies:
      - target: StintExtensionsCore
        embed: false
    settings:
      base:
        PRODUCT_NAME: StintIntentsExtension
        PRODUCT_BUNDLE_IDENTIFIER: tech.reyem.stint.intents
        WRAPPER_EXTENSION: appex
        SKIP_INSTALL: YES
        LD_RUNPATH_SEARCH_PATHS:
          - "@executable_path/../../Frameworks"
        SWIFT_OPTIMIZATION_LEVEL: "-Onone"
        OTHER_LDFLAGS:
          - "-Wl,-undefined,dynamic_lookup"
```

The `-undefined,dynamic_lookup` flag is required because RustFFI.swift declares external symbols (`stint_verb_*`, `stint_settings_*`, etc.) that get resolved at runtime from the host stint-app binary's flat namespace — same trick the legacy framework uses.

- [ ] **Step 2: Regenerate + build**

```bash
cd crates/stint-app/swift/xcodegen
xcodegen generate 2>&1 | tail -3
xcodebuild build -scheme StintIntentsExtension -configuration Release -destination 'platform=macOS' -derivedDataPath ./build/derived 2>&1 | tail -5
cd -
```

Expected: `** BUILD SUCCEEDED **`.

- [ ] **Step 3: Verify the .appex shape**

```bash
APPEX="crates/stint-app/swift/xcodegen/build/derived/Build/Products/Release/StintIntentsExtension.appex"
ls "$APPEX/Contents/" && file "$APPEX/Contents/MacOS/StintIntentsExtension"
ls "$APPEX/Contents/Resources/Metadata.appintents/" 2>/dev/null && echo "✓ stencil present"
```

Expected: `Info.plist`, `MacOS/`, `Frameworks/`, `Resources/Metadata.appintents/` directories. The Metadata.appintents stencil is critical — that's what Apple's intent indexer reads.

- [ ] **Step 4: Commit**

```bash
git add crates/stint-app/swift/xcodegen/project.yml
git commit -m "feat(6d): StintIntentsExtension target — produces real App Intents .appex"
```

---

## Task B5: Add build_stint_intents_extension() to build.rs

**Files:**
- Modify: `crates/stint-app/build.rs`

- [ ] **Step 1: Add the new build function**

Open `crates/stint-app/build.rs`. After the `build_stint_widget()` function (Phase A replaced its body), append a new function:

```rust
/// Build the StintIntentsExtension app extension via xcodegen + xcodebuild
/// and place the resulting `.appex` bundle at
/// `crates/stint-app/PlugIns/StintIntentsExtension.appex/`.
fn build_stint_intents_extension() -> Result<(), String> {
    if env::var_os("STINT_SKIP_SWIFT_BUILD").is_some_and(|v| !v.is_empty()) {
        return Err("STINT_SKIP_SWIFT_BUILD is set".into());
    }
    if env::var_os("CARGO_CFG_TARGET_OS").is_some_and(|v| v != "macos") {
        return Err("non-macOS target".into());
    }

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").map_err(|e| e.to_string())?;
    let xcodegen_dir = Path::new(&manifest_dir).join("swift/xcodegen");
    let project_yml = xcodegen_dir.join("project.yml");
    if !project_yml.exists() {
        return Err(format!("missing {}", project_yml.display()));
    }

    let ext_dir = Path::new(&manifest_dir).join("swift/Extensions/StintIntentsExtension");
    if let Ok(entries) = fs::read_dir(&ext_dir) {
        for entry in entries.flatten() {
            print_rerun_if_changed_recursive(&entry.path());
        }
    }

    // xcodegen generate is idempotent; build_stint_widget() already runs it
    // earlier in main(), but call again to be safe in case build order changes.
    let xcgen = Command::new("xcodegen")
        .current_dir(&xcodegen_dir)
        .arg("generate")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| format!("xcodegen spawn: {e}"))?;
    if !xcgen.success() {
        return Err(format!("xcodegen exit {xcgen}"));
    }

    let derived_data = xcodegen_dir.join("build/derived");
    let status = Command::new("xcodebuild")
        .current_dir(&xcodegen_dir)
        .args([
            "-scheme",
            "StintIntentsExtension",
            "-configuration",
            "Release",
            "-destination",
            "platform=macOS",
            "-derivedDataPath",
            derived_data.to_str().ok_or("derived path not utf8")?,
            "build",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| format!("xcodebuild spawn: {e}"))?;
    if !status.success() {
        return Err(format!("xcodebuild exit {status}"));
    }

    let built = derived_data.join("Build/Products/Release/StintIntentsExtension.appex");
    if !built.exists() {
        return Err(format!("missing {}", built.display()));
    }

    let dest = Path::new(&manifest_dir).join("PlugIns/StintIntentsExtension.appex");
    let _ = fs::remove_dir_all(&dest);
    fs::create_dir_all(dest.parent().unwrap()).map_err(|e| format!("create PlugIns/: {e}"))?;
    copy_dir(&built, &dest).map_err(|e| format!("copy appex: {e}"))?;

    codesign_adhoc(&dest).map_err(|e| format!("codesign appex: {e}"))?;

    println!(
        "cargo:warning=StintIntentsExtension.appex rebuilt at {}",
        dest.display()
    );
    Ok(())
}
```

- [ ] **Step 2: Wire into main()**

Edit `main()` in build.rs. After the existing `build_stint_widget()` call, add:

```rust
    if let Err(e) = build_stint_intents_extension() {
        println!("cargo:warning=StintIntentsExtension appex build skipped: {e}");
    }
```

Final `main()` should look like:

```rust
fn main() {
    if let Err(e) = build_stint_intents_framework() {
        println!("cargo:warning=StintIntents framework build skipped: {e}");
    }
    if let Err(e) = build_stint_widget() {
        println!("cargo:warning=StintWidget appex build skipped: {e}");
    }
    if let Err(e) = build_stint_intents_extension() {
        println!("cargo:warning=StintIntentsExtension appex build skipped: {e}");
    }
    tauri_build::build()
}
```

Note we keep `build_stint_intents_framework()` running — the legacy framework still provides Spotlight indexing via dlsym throughout Phase B. Phase D removes it.

- [ ] **Step 3: Cargo build to verify**

```bash
cargo build -p stint-app 2>&1 | tail -8
```

Expected: three `cargo:warning=… rebuilt at …` lines (framework, widget, intents extension) and a `Finished` line.

- [ ] **Step 4: Verify the bundle**

```bash
ls crates/stint-app/PlugIns/
file crates/stint-app/PlugIns/StintIntentsExtension.appex/Contents/MacOS/StintIntentsExtension
```

Expected: both `StintIntentsExtension.appex/` and `StintWidget.appex/` directories. Binary is `Mach-O 64-bit executable arm64`.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-app/build.rs
git commit -m "build(6d): build.rs drives StintIntentsExtension.appex production"
```

---

## Task B6: Bundle StintIntentsExtension.appex into Stint.app via wrapper script

**Files:**
- Modify: `scripts/build-app-with-widget.sh`

- [ ] **Step 1: Generalize the wrapper to relocate + sign two .appex bundles**

Open `scripts/build-app-with-widget.sh`. Replace the entire body of the script with this updated version that handles both extensions:

```bash
#!/usr/bin/env bash
# Build Stint.app and relocate the embedded extension .appex bundles into
# Contents/PlugIns/ where macOS's WidgetKit + App Intents indexer look for
# them. Tauri's bundle.resources puts files under Contents/Resources/ but
# Apple requires extensions at Contents/PlugIns/<name>.appex.
#
# Phase 6d: ships TWO extensions — StintWidget + StintIntentsExtension.
#
# Usage:
#   scripts/build-app-with-widget.sh             # ad-hoc sign (local dev install)
#   scripts/build-app-with-widget.sh "Developer ID Application: ..."  # release sign

set -euo pipefail

readonly REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

readonly SIGN_IDENTITY="${1:--}"
readonly APP="target/release/bundle/macos/Stint.app"

echo "==> Building Stint.app"
cargo tauri build --bundles app

relocate_appex() {
  local name="$1"
  local src="crates/stint-app/PlugIns/${name}.appex"
  local dest="$APP/Contents/PlugIns/${name}.appex"
  if [[ ! -d "$src" ]]; then
    echo "ERROR: $src missing — build.rs did not produce $name.appex"
    exit 1
  fi
  echo "==> Relocating ${name}.appex into Contents/PlugIns/"
  mkdir -p "$(dirname "$dest")"
  rm -rf "$dest"
  cp -R "$src" "$dest"
}

relocate_appex StintWidget
relocate_appex StintIntentsExtension

# Strip the Resources/PlugIns duplicate Tauri may leave behind.
rm -rf "$APP/Contents/Resources/PlugIns"

echo "==> Re-signing embedded StintIntents framework (build.rs ad-hoc only)"
codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" \
  "$APP/Contents/Frameworks/StintIntents.framework"

echo "==> Signing StintWidget.appex with $SIGN_IDENTITY"
codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" \
  --entitlements crates/stint-app/swift/Extensions/StintWidget/StintWidget.entitlements \
  "$APP/Contents/PlugIns/StintWidget.appex"

echo "==> Signing StintIntentsExtension.appex with $SIGN_IDENTITY"
codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" \
  --entitlements crates/stint-app/swift/Extensions/StintIntentsExtension/StintIntentsExtension.entitlements \
  "$APP/Contents/PlugIns/StintIntentsExtension.appex"

echo "==> Re-signing main bundle to seal the new PlugIns/ + Frameworks/"
codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" \
  --entitlements crates/stint-app/entitlements.plist \
  "$APP/Contents/MacOS/stint-app"
codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" \
  --entitlements crates/stint-app/entitlements.plist \
  "$APP"

echo "==> Verifying signature"
codesign --verify --deep --strict --verbose=2 "$APP" 2>&1 | tail -3

echo "==> Done. Bundle at $APP"
```

- [ ] **Step 2: Run the wrapper with ad-hoc sign**

```bash
scripts/build-app-with-widget.sh 2>&1 | tail -15
```

Expected: both .appex bundles listed in the relocation output; `codesign --verify` passes.

- [ ] **Step 3: Verify both bundles landed in /Contents/PlugIns/**

```bash
ls target/release/bundle/macos/Stint.app/Contents/PlugIns/
```

Expected: `StintIntentsExtension.appex StintWidget.appex`.

- [ ] **Step 4: Commit**

```bash
git add scripts/build-app-with-widget.sh
git commit -m "build(6d): wrapper script bundles + signs both extension appex bundles"
```

---

## Task B7: Notarize + install + verify Shortcuts.app discovery

**Files:** none.

- [ ] **Step 1: Sign + notarize + staple + install**

```bash
scripts/build-app-with-widget.sh "Developer ID Application: Reyem Technologies Inc. (WAK5K2758P)"

APP="target/release/bundle/macos/Stint.app"
ZIP="${APP}.zip"
rm -f "$ZIP"
ditto -c -k --keepParent "$APP" "$ZIP"
xcrun notarytool submit "$ZIP" --keychain-profile "stint-notary" --wait

xcrun stapler staple "$APP"
killall stint-app 2>/dev/null; sleep 1
rm -rf /Applications/Stint.app
cp -R "$APP" /Applications/
xattr -cr /Applications/Stint.app
/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister -f /Applications/Stint.app
open /Applications/Stint.app
sleep 10  # extra time for siriactionsd to ingest the new stencil
```

Expected: notarization `status: Accepted`; staple worked.

- [ ] **Step 2: Verify pluginkit + Shortcuts.app**

```bash
pluginkit -m -p com.apple.appintents-extension | grep -i stint
pluginkit -m -p com.apple.widgetkit-extension | grep -i stint
```

Expected: both queries list the corresponding `tech.reyem.stint.*` bundle IDs.

- [ ] **Step 3: Manually verify Shortcuts.app**

1. Open Shortcuts.app.
2. Click `+` → search "stint".
3. Expect actions: Start Timer / Stop Timer / Current / List Today / Switch Project / Update Entry / etc.
4. Drag "Start Timer" into a new shortcut. The action's parameter UI should render (project picker, description field).

- [ ] **Step 4: Manually verify Spotlight unchanged**

The legacy framework still handles Spotlight indexing in Phase B. Verify no regression:

1. Start a timer in stint with description "phase-b-spotlight-test".
2. ⌘-Space → "phase-b-spotlight-test" → entry result should appear within a few seconds.

If Spotlight regressed, the most likely cause is that the App Intents Extension also tried to register the same intent types and confused the indexer. Check Console.app filtered by `subsystem:com.apple.appintents` for collision messages.

- [ ] **Step 5: Commit verification marker**

```bash
git commit --allow-empty -m "test(6d): Phase B — App Intents Extension discovered by Shortcuts.app

pluginkit lists tech.reyem.stint.intents under com.apple.appintents-
extension. Shortcuts.app search 'stint' shows the full intent catalog;
parameter pickers render. Legacy framework Spotlight path unchanged."
```

---

# Phase C — Move SpotlightIndexer + IPC

Goal: move Spotlight indexing from the legacy in-process framework path to the new App Intents Extension via App Group container + Darwin notifications. End state: mutating an entry in stint-app triggers a Spotlight reindex within ~10 seconds.

---

## Task C1: Add App Group entitlement to host stint-app

**Files:**
- Modify: `crates/stint-app/entitlements.plist`

- [ ] **Step 1: Add the App Group key**

Open `crates/stint-app/entitlements.plist`. Inside the top-level `<dict>`, add:

```xml
    <key>com.apple.security.application-groups</key>
    <array>
        <string>group.tech.reyem.stint</string>
    </array>
```

The full file should now contain (in addition to whatever's already there) those keys above the closing `</dict></plist>`.

- [ ] **Step 2: Verify XML is well-formed**

```bash
plutil -lint crates/stint-app/entitlements.plist
```

Expected: `OK`.

- [ ] **Step 3: Commit**

```bash
git add crates/stint-app/entitlements.plist
git commit -m "feat(6d): host entitlements — App Group for Spotlight IPC"
```

---

## Task C2: Add App Group entitlement to both extension entitlements

**Files:**
- Modify: `crates/stint-app/swift/Extensions/StintWidget/StintWidget.entitlements`
- Modify: `crates/stint-app/swift/Extensions/StintIntentsExtension/StintIntentsExtension.entitlements`

- [ ] **Step 1: Add App Group to widget entitlements**

Replace the entire contents of `crates/stint-app/swift/Extensions/StintWidget/StintWidget.entitlements` with:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.app-sandbox</key>
    <true/>
    <key>com.apple.security.network.client</key>
    <true/>
    <key>com.apple.security.application-groups</key>
    <array>
        <string>group.tech.reyem.stint</string>
    </array>
</dict>
</plist>
```

- [ ] **Step 2: Add App Group to intents extension entitlements**

Replace the entire contents of `crates/stint-app/swift/Extensions/StintIntentsExtension/StintIntentsExtension.entitlements` with:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.app-sandbox</key>
    <true/>
    <key>com.apple.security.network.client</key>
    <true/>
    <key>com.apple.security.application-groups</key>
    <array>
        <string>group.tech.reyem.stint</string>
    </array>
</dict>
</plist>
```

- [ ] **Step 3: Lint both**

```bash
plutil -lint crates/stint-app/swift/Extensions/StintWidget/StintWidget.entitlements
plutil -lint crates/stint-app/swift/Extensions/StintIntentsExtension/StintIntentsExtension.entitlements
```

Expected: `OK` for both.

- [ ] **Step 4: Commit**

```bash
git add crates/stint-app/swift/Extensions/StintWidget/StintWidget.entitlements \
        crates/stint-app/swift/Extensions/StintIntentsExtension/StintIntentsExtension.entitlements
git commit -m "feat(6d): App Group entitlement on both extension targets"
```

---

## Task C3: Write SharedContainerMarker.swift + tests (TDD)

**Files:**
- Create: `crates/stint-app/swift/StintExtensionsCore/Sources/IPC/SharedContainerMarker.swift`
- Create: `crates/stint-app/swift/StintExtensionsCore/Tests/SharedContainerMarkerTests.swift`

- [ ] **Step 1: Write the failing test**

```bash
mkdir -p crates/stint-app/swift/StintExtensionsCore/Sources/IPC
cat > crates/stint-app/swift/StintExtensionsCore/Tests/SharedContainerMarkerTests.swift <<'EOF'
import XCTest
import Foundation
@testable import StintExtensionsCore

final class SharedContainerMarkerTests: XCTestCase {
    var tempDir: URL!

    override func setUp() {
        tempDir = FileManager.default.temporaryDirectory
            .appendingPathComponent("marker-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
    }

    override func tearDown() {
        try? FileManager.default.removeItem(at: tempDir)
    }

    func testEmptyOnFirstRead() throws {
        let marker = SharedContainerMarker(containerOverride: tempDir)
        XCTAssertEqual(try marker.drain(), [])
    }

    func testAppendThenDrain() throws {
        let marker = SharedContainerMarker(containerOverride: tempDir)
        try marker.append(SpotlightOp(localUuid: "u1", kind: .entryStarted))
        try marker.append(SpotlightOp(localUuid: "u2", kind: .entryDeleted))

        let drained = try marker.drain()
        XCTAssertEqual(drained.count, 2)
        XCTAssertEqual(drained[0].localUuid, "u1")
        XCTAssertEqual(drained[0].kind, .entryStarted)
        XCTAssertEqual(drained[1].localUuid, "u2")
        XCTAssertEqual(drained[1].kind, .entryDeleted)
    }

    func testDrainClearsFile() throws {
        let marker = SharedContainerMarker(containerOverride: tempDir)
        try marker.append(SpotlightOp(localUuid: "u1", kind: .entryStarted))
        _ = try marker.drain()
        XCTAssertEqual(try marker.drain(), [])
    }
}
EOF
```

- [ ] **Step 2: Run the test to confirm it fails (no SharedContainerMarker yet)**

```bash
cd crates/stint-app/swift/xcodegen
xcodegen generate >/dev/null
xcodebuild test -scheme StintExtensionsCoreTests -destination 'platform=macOS' -derivedDataPath ./build/derived 2>&1 | tail -5
cd -
```

Expected: compile error — `cannot find 'SharedContainerMarker' in scope`.

- [ ] **Step 3: Write the implementation**

```bash
cat > crates/stint-app/swift/StintExtensionsCore/Sources/IPC/SharedContainerMarker.swift <<'EOF'
import Foundation

/// One pending Spotlight operation queued by the host for the extension to
/// process. `kind` mirrors the legacy `IndexerKind` enum cases from Rust's
/// `stint-core/src/ffi.rs` 1:1 so the existing SpotlightIndexer.delta()
/// machinery can consume them without semantic loss.
public struct SpotlightOp: Codable, Equatable {
    public let localUuid: String
    public let kind: Kind

    public enum Kind: String, Codable, Equatable {
        case entryStarted
        case entryStopped
        case entryUpdated
        case entryDeleted
        case projectsReplaced
        case tasksReplaced
    }

    public init(localUuid: String, kind: Kind) {
        self.localUuid = localUuid
        self.kind = kind
    }
}

/// Append-only JSON marker file in the App Group shared container. The host
/// appends mutations; the extension drains them on Darwin notification or
/// at next wake.
///
/// Container path:
///   ~/Library/Group Containers/group.tech.reyem.stint/pending-reindex.json
///
/// Use `containerOverride` in tests to write to a tempdir instead.
public final class SharedContainerMarker {
    public static let appGroupId = "group.tech.reyem.stint"
    public static let fileName = "pending-reindex.json"

    private let containerURL: URL

    public init(containerOverride: URL? = nil) {
        if let override = containerOverride {
            self.containerURL = override
        } else if let group = FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: Self.appGroupId) {
            self.containerURL = group
        } else {
            // App Group not entitled (CLI / dev binary). Fall back to a
            // per-process tempdir so calls don't crash; the data won't be
            // visible across processes but tests of producer-side behavior
            // still work.
            self.containerURL = FileManager.default.temporaryDirectory
                .appendingPathComponent("stint-marker-fallback")
            try? FileManager.default.createDirectory(at: self.containerURL, withIntermediateDirectories: true)
        }
    }

    private var fileURL: URL {
        containerURL.appendingPathComponent(Self.fileName)
    }

    /// Append one operation. Atomic via write-temp + rename.
    public func append(_ op: SpotlightOp) throws {
        var existing = (try? loadOps()) ?? []
        existing.append(op)
        try writeOps(existing)
    }

    /// Read all pending ops and clear the file.
    public func drain() throws -> [SpotlightOp] {
        guard FileManager.default.fileExists(atPath: fileURL.path) else { return [] }
        let ops = try loadOps()
        try writeOps([])
        return ops
    }

    private func loadOps() throws -> [SpotlightOp] {
        let data = try Data(contentsOf: fileURL)
        if data.isEmpty { return [] }
        return try JSONDecoder().decode([SpotlightOp].self, from: data)
    }

    private func writeOps(_ ops: [SpotlightOp]) throws {
        let data = try JSONEncoder().encode(ops)
        let tmp = fileURL.appendingPathExtension("tmp")
        try data.write(to: tmp, options: .atomic)
        _ = try FileManager.default.replaceItemAt(fileURL, withItemAt: tmp)
    }
}
EOF
```

- [ ] **Step 4: Run tests to confirm they pass**

```bash
cd crates/stint-app/swift/xcodegen
xcodegen generate >/dev/null
xcodebuild test -scheme StintExtensionsCoreTests -destination 'platform=macOS' -derivedDataPath ./build/derived 2>&1 | tail -8
cd -
```

Expected: `** TEST SUCCEEDED **`, all 3 SharedContainerMarker tests pass alongside the existing 5.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-app/swift/StintExtensionsCore/Sources/IPC/ \
        crates/stint-app/swift/StintExtensionsCore/Tests/SharedContainerMarkerTests.swift
git commit -m "feat(6d): SharedContainerMarker — append/drain pending Spotlight ops"
```

---

## Task C4: Write DarwinNotification.swift + tests (TDD)

**Files:**
- Create: `crates/stint-app/swift/StintExtensionsCore/Sources/IPC/DarwinNotification.swift`
- Create: `crates/stint-app/swift/StintExtensionsCore/Tests/DarwinNotificationTests.swift`

- [ ] **Step 1: Write the failing test**

```bash
cat > crates/stint-app/swift/StintExtensionsCore/Tests/DarwinNotificationTests.swift <<'EOF'
import XCTest
import Foundation
@testable import StintExtensionsCore

final class DarwinNotificationTests: XCTestCase {
    func testPostAndObserveRoundTrip() {
        let name = "tech.reyem.stint.test.\(UUID().uuidString)"
        let received = expectation(description: "observer fires")

        let token = DarwinNotification.observe(name: name) {
            received.fulfill()
        }

        DarwinNotification.post(name: name)
        wait(for: [received], timeout: 2.0)

        DarwinNotification.removeObserver(token)
    }
}
EOF
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cd crates/stint-app/swift/xcodegen
xcodegen generate >/dev/null
xcodebuild test -scheme StintExtensionsCoreTests -destination 'platform=macOS' -derivedDataPath ./build/derived 2>&1 | tail -5
cd -
```

Expected: compile error — `cannot find 'DarwinNotification' in scope`.

- [ ] **Step 3: Write the implementation**

```bash
cat > crates/stint-app/swift/StintExtensionsCore/Sources/IPC/DarwinNotification.swift <<'EOF'
import Foundation
import CoreFoundation

/// Thin wrapper around CFNotificationCenter's Darwin notification API. Used
/// to wake the App Intents Extension when the host has new Spotlight work
/// queued in the shared container.
///
/// The canonical notification name is `tech.reyem.stint.reindex`.
public enum DarwinNotification {
    public static let reindexName = "tech.reyem.stint.reindex"

    /// Token returned by `observe` so the caller can pass it to
    /// `removeObserver` on teardown.
    public final class Token {
        let name: CFString
        let opaque: UnsafeRawPointer
        init(name: CFString, opaque: UnsafeRawPointer) {
            self.name = name
            self.opaque = opaque
        }
    }

    /// Post a Darwin notification. Cross-process; no payload.
    public static func post(name: String) {
        let center = CFNotificationCenterGetDarwinNotifyCenter()
        let cfName = name as CFString
        CFNotificationCenterPostNotification(center, CFNotificationName(cfName), nil, nil, true)
    }

    /// Register an observer. The callback is invoked on the main queue. Returns
    /// a token; pass it to `removeObserver` when done.
    @discardableResult
    public static func observe(name: String, callback: @escaping () -> Void) -> Token {
        let center = CFNotificationCenterGetDarwinNotifyCenter()
        let cfName = name as CFString

        let box = Box(callback: callback)
        let opaque = Unmanaged.passRetained(box).toOpaque()

        CFNotificationCenterAddObserver(
            center,
            opaque,
            { _, observer, _, _, _ in
                guard let observer else { return }
                let box = Unmanaged<Box>.fromOpaque(observer).takeUnretainedValue()
                DispatchQueue.main.async { box.callback() }
            },
            cfName,
            nil,
            .deliverImmediately
        )

        return Token(name: cfName, opaque: UnsafeRawPointer(opaque))
    }

    public static func removeObserver(_ token: Token) {
        let center = CFNotificationCenterGetDarwinNotifyCenter()
        CFNotificationCenterRemoveObserver(center, token.opaque, CFNotificationName(token.name), nil)
        Unmanaged<Box>.fromOpaque(UnsafeMutableRawPointer(mutating: token.opaque)).release()
    }

    private final class Box {
        let callback: () -> Void
        init(callback: @escaping () -> Void) { self.callback = callback }
    }
}
EOF
```

- [ ] **Step 4: Run tests to confirm they pass**

```bash
cd crates/stint-app/swift/xcodegen
xcodegen generate >/dev/null
xcodebuild test -scheme StintExtensionsCoreTests -destination 'platform=macOS' -derivedDataPath ./build/derived 2>&1 | tail -5
cd -
```

Expected: `** TEST SUCCEEDED **`. Total tests now: 5 (legacy) + 3 (SharedContainerMarker) + 1 (DarwinNotification) = 9.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-app/swift/StintExtensionsCore/Sources/IPC/DarwinNotification.swift \
        crates/stint-app/swift/StintExtensionsCore/Tests/DarwinNotificationTests.swift
git commit -m "feat(6d): DarwinNotification — post + observe wrapper for host↔extension wakeup"
```

---

## Task C5: Copy SpotlightIndexer + Focus + ActivityTracker into StintExtensionsCore

**Files:**
- Create: `crates/stint-app/swift/StintExtensionsCore/Sources/Spotlight/SpotlightIndexer.swift` (copy)
- Create: `crates/stint-app/swift/StintExtensionsCore/Sources/Spotlight/ActivityTracker.swift` (copy)
- Create: `crates/stint-app/swift/StintExtensionsCore/Sources/Focus/ProjectFocusFilter.swift` (copy)
- Create: `crates/stint-app/swift/StintExtensionsCore/Sources/Init/StintIntentsInit.swift` (copy)

- [ ] **Step 1: Copy the files**

```bash
SRC=crates/stint-app/swift/StintIntents/Sources/StintIntents
DST=crates/stint-app/swift/StintExtensionsCore/Sources

mkdir -p $DST/Spotlight $DST/Focus $DST/Init
cp $SRC/Spotlight/SpotlightIndexer.swift $DST/Spotlight/
cp $SRC/Spotlight/ActivityTracker.swift $DST/Spotlight/
cp $SRC/Focus/ProjectFocusFilter.swift $DST/Focus/
cp $SRC/Init/StintIntentsInit.swift $DST/Init/
```

- [ ] **Step 2: Regenerate + build the framework**

```bash
cd crates/stint-app/swift/xcodegen
xcodegen generate 2>&1 | tail -3
xcodebuild build -scheme StintExtensionsCore -configuration Release -destination 'platform=macOS' -derivedDataPath ./build/derived 2>&1 | tail -5
cd -
```

Expected: `** BUILD SUCCEEDED **`.

If compile errors appear (e.g. missing `Bridge` references), the most likely cause is that `SpotlightIndexer.swift` imports something from the legacy `Bridge.swift` that needs to use the new `Bridge/RustFFI.swift`. Open the failing file, change the symbol references to match the renamed module location (the Swift code is the same; only the file path changed).

- [ ] **Step 3: Commit**

```bash
git add crates/stint-app/swift/StintExtensionsCore/Sources/{Spotlight,Focus,Init}/
git commit -m "chore(6d): copy SpotlightIndexer + Focus + Init into StintExtensionsCore"
```

---

## Task C6: Write Rust spotlight_ipc helper + tests (TDD)

**Files:**
- Create: `crates/stint-app/src/spotlight_ipc.rs`
- Create: `crates/stint-app/tests/spotlight_ipc.rs`
- Modify: `crates/stint-app/src/lib.rs`

- [ ] **Step 1: Write the failing integration test**

```bash
cat > crates/stint-app/tests/spotlight_ipc.rs <<'EOF'
//! Integration test for the Rust-side Spotlight IPC helper. Uses
//! STINT_APP_GROUP_OVERRIDE_DIR to redirect writes to a tempdir so the
//! real App Group container isn't touched.

use std::env;
use stint_app::spotlight_ipc::{push_pending, SpotlightOp};
use tempfile::TempDir;

struct EnvRestore {
    key: String,
    prev: Option<String>,
}
impl EnvRestore {
    fn set(key: &str, value: &str) -> Self {
        let prev = env::var(key).ok();
        env::set_var(key, value);
        Self { key: key.into(), prev }
    }
}
impl Drop for EnvRestore {
    fn drop(&mut self) {
        match &self.prev {
            Some(v) => env::set_var(&self.key, v),
            None => env::remove_var(&self.key),
        }
    }
}

#[test]
fn push_pending_writes_marker_file() {
    let dir = TempDir::new().unwrap();
    let _guard = EnvRestore::set("STINT_APP_GROUP_OVERRIDE_DIR", dir.path().to_str().unwrap());

    push_pending("uuid-A", SpotlightOp::EntryStarted).unwrap();
    push_pending("uuid-B", SpotlightOp::EntryDeleted).unwrap();

    let marker_path = dir.path().join("pending-reindex.json");
    assert!(marker_path.exists());
    let content = std::fs::read_to_string(marker_path).unwrap();
    assert!(content.contains("uuid-A"));
    assert!(content.contains("uuid-B"));
    assert!(content.contains("entryStarted"));
    assert!(content.contains("entryDeleted"));
}
EOF
```

- [ ] **Step 2: Run, confirm fail**

```bash
cargo test -p stint-app --test spotlight_ipc 2>&1 | tail -5
```

Expected: compile error — `stint_app::spotlight_ipc` not found.

- [ ] **Step 3: Implement the helper**

Create `crates/stint-app/src/spotlight_ipc.rs`:

```rust
//! Host → App Intents Extension IPC for Spotlight reindex.
//!
//! Replaces the in-process dlsym path that the legacy StintIntents
//! framework used. Verb call sites push pending ops here; the extension
//! drains them on its next wake.
//!
//! Storage shape mirrors Swift's `SharedContainerMarker` in
//! `StintExtensionsCore/Sources/IPC/SharedContainerMarker.swift`.

use std::env;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const APP_GROUP_ID: &str = "group.tech.reyem.stint";
const FILE_NAME: &str = "pending-reindex.json";
const DARWIN_NOTIFICATION: &str = "tech.reyem.stint.reindex";

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SpotlightOp {
    EntryStarted,
    EntryStopped,
    EntryUpdated,
    EntryDeleted,
    ProjectsReplaced,
    TasksReplaced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingEntry {
    #[serde(rename = "localUuid")]
    local_uuid: String,
    kind: SpotlightOp,
}

fn container_dir() -> PathBuf {
    if let Ok(override_dir) = env::var("STINT_APP_GROUP_OVERRIDE_DIR") {
        return PathBuf::from(override_dir);
    }
    // ~/Library/Group Containers/<APP_GROUP_ID>/
    let home = env::var("HOME").unwrap_or_default();
    PathBuf::from(home)
        .join("Library/Group Containers")
        .join(APP_GROUP_ID)
}

/// Append a pending op to the shared container marker file and post the
/// Darwin notification so the extension wakes up eagerly. Best-effort:
/// errors are returned but call sites typically log-and-continue.
pub fn push_pending(local_uuid: &str, op: SpotlightOp) -> std::io::Result<()> {
    let dir = container_dir();
    fs::create_dir_all(&dir)?;
    let path = dir.join(FILE_NAME);

    let mut entries: Vec<PendingEntry> = if path.exists() {
        let data = fs::read(&path)?;
        if data.is_empty() {
            Vec::new()
        } else {
            serde_json::from_slice(&data).unwrap_or_default()
        }
    } else {
        Vec::new()
    };
    entries.push(PendingEntry {
        local_uuid: local_uuid.into(),
        kind: op,
    });

    let json = serde_json::to_vec(&entries)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, &json)?;
    fs::rename(&tmp, &path)?;

    #[cfg(target_os = "macos")]
    post_darwin_notification();

    Ok(())
}

#[cfg(target_os = "macos")]
fn post_darwin_notification() {
    use core_foundation::base::TCFType;
    use core_foundation::notification_center::CFNotificationCenter;
    use core_foundation::string::CFString;

    let name = CFString::new(DARWIN_NOTIFICATION);
    let center = CFNotificationCenter::darwin_notify_center();
    center.post_notification(name, None::<&CFString>, false);
}

#[cfg(not(target_os = "macos"))]
fn post_darwin_notification() {}
```

- [ ] **Step 4: Add core-foundation dep to Cargo.toml**

Open `crates/stint-app/Cargo.toml`. In the `[dependencies]` section, add:

```toml
core-foundation = "0.10"
```

(Or whatever the latest 0.x is — check `cargo search core-foundation` if needed.)

- [ ] **Step 5: Wire the module into lib.rs**

Open `crates/stint-app/src/lib.rs`. Add a new line in alphabetical order with the other `pub mod` declarations:

```rust
pub mod spotlight_ipc;
```

- [ ] **Step 6: Run the test to verify it passes**

```bash
cargo test -p stint-app --test spotlight_ipc 2>&1 | tail -5
```

Expected: `test result: ok. 1 passed`.

- [ ] **Step 7: Commit**

```bash
git add crates/stint-app/src/spotlight_ipc.rs \
        crates/stint-app/src/lib.rs \
        crates/stint-app/Cargo.toml \
        crates/stint-app/Cargo.lock \
        crates/stint-app/tests/spotlight_ipc.rs
git commit -m "feat(6d): spotlight_ipc helper — append + Darwin post for extension wakeup"
```

---

## Task C7: Replace stint-core's notify_indexer with spotlight_ipc

**Files:**
- Modify: `crates/stint-core/src/ffi.rs`

The existing `notify_indexer()` in stint-core/src/ffi.rs uses dlsym to call into the Swift framework. Replace its body to write the marker file + post the Darwin notification instead. The function signature stays the same; the verb call sites don't change.

- [ ] **Step 1: Replace the notify_indexer implementation**

Open `crates/stint-core/src/ffi.rs`. Find `pub fn notify_indexer(kind: IndexerKind, payload_json: &str)` (around line 520). Replace its body so it writes to the App Group container instead of dlsym'ing.

```rust
/// Notify the Spotlight indexer about a mutation. As of Phase 6d this
/// writes to the App Group shared container at
/// `~/Library/Group Containers/group.tech.reyem.stint/pending-reindex.json`
/// and posts a Darwin notification. The App Intents Extension wakes on
/// the notification (or at its next scheduled wake) and drains the
/// pending ops into Spotlight's index.
///
/// Best-effort: errors are silently swallowed (CLI / headless / no
/// container entitlement). Call sites in the verbs façade don't need to
/// change.
pub fn notify_indexer(kind: IndexerKind, payload_json: &str) {
    // Extract local_uuid from the payload JSON for the marker file. Verbs
    // that mutate a single entry pass {"local_uuid": "..."}-shaped payloads;
    // ProjectsReplaced / TasksReplaced pass payloads without a UUID and we
    // record a sentinel.
    let local_uuid = serde_json::from_str::<serde_json::Value>(payload_json)
        .ok()
        .and_then(|v| {
            v.get("local_uuid")
                .and_then(|s| s.as_str())
                .map(String::from)
        })
        .unwrap_or_default();

    let op = match kind {
        IndexerKind::EntryStarted => "entryStarted",
        IndexerKind::EntryStopped => "entryStopped",
        IndexerKind::EntryUpdated => "entryUpdated",
        IndexerKind::EntryDeleted => "entryDeleted",
        IndexerKind::ProjectsReplaced => "projectsReplaced",
        IndexerKind::TasksReplaced => "tasksReplaced",
    };

    let _ = append_pending(&local_uuid, op);

    #[cfg(target_os = "macos")]
    {
        let name = "tech.reyem.stint.reindex\0";
        unsafe {
            CFNotificationCenterPostNotification(
                CFNotificationCenterGetDarwinNotifyCenter(),
                CFStringCreateWithCString(
                    std::ptr::null(),
                    name.as_ptr() as *const c_char,
                    0x08000100, // kCFStringEncodingUTF8
                ),
                std::ptr::null(),
                std::ptr::null(),
                1, // deliverImmediately
            );
        }
    }
}

fn append_pending(local_uuid: &str, op: &str) -> std::io::Result<()> {
    use std::fs;
    use std::path::PathBuf;

    let home = std::env::var("HOME").unwrap_or_default();
    let dir = PathBuf::from(home)
        .join("Library/Group Containers/group.tech.reyem.stint");
    fs::create_dir_all(&dir)?;
    let path = dir.join("pending-reindex.json");

    let mut entries: Vec<serde_json::Value> = if path.exists() {
        let data = fs::read(&path)?;
        if data.is_empty() {
            Vec::new()
        } else {
            serde_json::from_slice(&data).unwrap_or_default()
        }
    } else {
        Vec::new()
    };
    entries.push(serde_json::json!({
        "localUuid": local_uuid,
        "kind": op,
    }));
    let data = serde_json::to_vec(&entries).map_err(std::io::Error::other)?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, &data)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}
```

- [ ] **Step 2: Add the CFNotificationCenter extern decl**

Near the top of `crates/stint-core/src/ffi.rs`, find where `extern "C"` blocks live. Add:

```rust
#[cfg(target_os = "macos")]
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFNotificationCenterGetDarwinNotifyCenter() -> *mut std::ffi::c_void;
    fn CFNotificationCenterPostNotification(
        center: *mut std::ffi::c_void,
        name: *mut std::ffi::c_void,
        object: *const std::ffi::c_void,
        user_info: *const std::ffi::c_void,
        deliver_immediately: i32,
    );
    fn CFStringCreateWithCString(
        alloc: *const std::ffi::c_void,
        cstr: *const c_char,
        encoding: u32,
    ) -> *mut std::ffi::c_void;
}
```

If `c_char` isn't already imported at the top of the file, add `use std::ffi::c_char;` to the imports.

- [ ] **Step 3: Remove the now-dead dlsym lookup code**

In the same file, delete the entire `lookup_indexer_notify()` function, the `INDEXER_NOTIFY_SYMBOL: OnceLock<...>` static, and the `IndexerNotifyFn` typedef. These were only used by the old dlsym path.

- [ ] **Step 4: Build the workspace**

```bash
cargo build --workspace 2>&1 | tail -5
```

Expected: `Finished` (no errors).

- [ ] **Step 5: Run all stint-core tests**

```bash
cargo test -p stint-core -- --test-threads=1 2>&1 | tail -5
```

Expected: all green (the verb tests should still pass — they don't depend on the dlsym path).

- [ ] **Step 6: Commit**

```bash
git add crates/stint-core/src/ffi.rs
git commit -m "feat(6d): stint-core notify_indexer writes App Group marker + Darwin post

Replaces the in-process dlsym call into the StintIntents Swift framework
with cross-process IPC: append to ~/Library/Group Containers/group.tech.
reyem.stint/pending-reindex.json, then post the
tech.reyem.stint.reindex Darwin notification so the App Intents
Extension wakes eagerly.

Verb call sites don't change — same function signature, same enum."
```

---

## Task C8: Write ExtensionLifecycle.swift — extension-side drain loop

**Files:**
- Create: `crates/stint-app/swift/Extensions/StintIntentsExtension/Sources/ExtensionLifecycle.swift`

This file installs the Darwin observer at extension launch and drains the marker file via SpotlightIndexer. Apple wakes the extension on its own schedule too, so the observer is best-effort eagerness.

- [ ] **Step 1: Create the source file**

```bash
cat > crates/stint-app/swift/Extensions/StintIntentsExtension/Sources/ExtensionLifecycle.swift <<'EOF'
import Foundation
import StintExtensionsCore

/// Module initializer that runs the first time the extension binary is
/// loaded. Registers the Darwin observer and drains any pending ops that
/// accumulated while we were asleep.
///
/// Swift doesn't have a `dyld constructor` story for executable targets,
/// so we use a static `let` whose initializer side-effects do the
/// registration. AppIntentsExtension's @main bootstrap touches the type
/// during launch, which triggers this initializer.
public enum ExtensionLifecycle {
    public static let _bootstrap: Void = {
        // Drain whatever's already queued at launch.
        drainPending()

        // Register the Darwin notification observer; calls drainPending()
        // again on each post.
        DarwinNotification.observe(name: DarwinNotification.reindexName) {
            drainPending()
        }
    }()

    private static func drainPending() {
        let marker = SharedContainerMarker()
        let ops = (try? marker.drain()) ?? []
        guard !ops.isEmpty else { return }

        let indexer = SpotlightIndexer()
        for op in ops {
            indexer.apply(op)
        }
    }
}
EOF
```

- [ ] **Step 2: Wire the bootstrap touch into IntentsExtensionMain.swift**

Update `crates/stint-app/swift/Extensions/StintIntentsExtension/Sources/IntentsExtensionMain.swift` to:

```bash
cat > crates/stint-app/swift/Extensions/StintIntentsExtension/Sources/IntentsExtensionMain.swift <<'EOF'
import AppIntents
import StintExtensionsCore

@main
struct StintAppIntentsExtension: AppIntentsExtension {
    init() {
        // Touch the bootstrap to trigger Darwin observer registration +
        // drain of any pending Spotlight ops.
        _ = ExtensionLifecycle._bootstrap
    }
}
EOF
```

- [ ] **Step 3: Add the apply(_ op:) method to SpotlightIndexer**

`SpotlightIndexer.swift` was copied from the legacy framework in Task C5. The legacy class has `delta(kind:payload:)`-style methods. Add a thin adapter that accepts the new `SpotlightOp` shape. Open `crates/stint-app/swift/StintExtensionsCore/Sources/Spotlight/SpotlightIndexer.swift` and append (inside the class):

```swift
    /// Phase 6d entry point: apply one queued op pulled from the App
    /// Group marker file. Adapts the new `SpotlightOp` shape to the
    /// existing `delta(kind:payload:)` API.
    public func apply(_ op: SpotlightOp) {
        let kind: IndexerKind
        switch op.kind {
        case .entryStarted: kind = .entryStarted
        case .entryStopped: kind = .entryStopped
        case .entryUpdated: kind = .entryUpdated
        case .entryDeleted: kind = .entryDeleted
        case .projectsReplaced: kind = .projectsReplaced
        case .tasksReplaced: kind = .tasksReplaced
        }
        let payload = #"{"local_uuid":"\#(op.localUuid)"}"#
        self.delta(kind: kind, payload: payload)
    }
```

If `SpotlightIndexer` is a struct (not a class), or if `delta(kind:payload:)` has a different name/signature in your copy, adapt accordingly — the goal is to route one queued op into the existing indexer machinery.

- [ ] **Step 4: Regenerate + build the extension**

```bash
cd crates/stint-app/swift/xcodegen
xcodegen generate >/dev/null
xcodebuild build -scheme StintIntentsExtension -configuration Release -destination 'platform=macOS' -derivedDataPath ./build/derived 2>&1 | tail -5
cd -
```

Expected: `** BUILD SUCCEEDED **`.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-app/swift/Extensions/StintIntentsExtension/Sources/ \
        crates/stint-app/swift/StintExtensionsCore/Sources/Spotlight/SpotlightIndexer.swift
git commit -m "feat(6d): ExtensionLifecycle drains marker + observes Darwin reindex"
```

---

## Task C9: Manual smoke — verify cross-process Spotlight reindex

**Files:** none.

- [ ] **Step 1: Build + notarize + install**

```bash
scripts/build-app-with-widget.sh "Developer ID Application: Reyem Technologies Inc. (WAK5K2758P)"

APP="target/release/bundle/macos/Stint.app"
ZIP="${APP}.zip"
rm -f "$ZIP"
ditto -c -k --keepParent "$APP" "$ZIP"
xcrun notarytool submit "$ZIP" --keychain-profile "stint-notary" --wait
xcrun stapler staple "$APP"

killall stint-app 2>/dev/null; sleep 1
rm -rf /Applications/Stint.app
cp -R "$APP" /Applications/
xattr -cr /Applications/Stint.app
/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister -f /Applications/Stint.app
open /Applications/Stint.app
sleep 5
```

- [ ] **Step 2: Trigger an entry mutation + watch the marker file**

```bash
# In one terminal — tail the marker file
watch -n 1 'ls -la ~/Library/Group\ Containers/group.tech.reyem.stint/ 2>&1; echo "---"; cat ~/Library/Group\ Containers/group.tech.reyem.stint/pending-reindex.json 2>&1'
```

In a second terminal, start a timer with a unique description:

```bash
/Applications/Stint.app/Contents/MacOS/stint start --description "phase-c-smoke-$(date +%s)"
```

Expected (in the watch terminal): within ~1s, `pending-reindex.json` appears with one entry. Within ~10s, it's drained (file becomes `[]`).

- [ ] **Step 3: Verify Spotlight surfaces the new entry**

After 10s: ⌘-Space, search the unique description string. Expect the stint entry result to appear.

If it doesn't appear within 30s:
- Check Console.app filtered by `process:StintIntentsExtension` for crashes.
- Check that the extension is actually running: `pgrep -lf StintIntentsExtension` (it may not be — Apple wakes extensions on demand). Touch the App Intents indexer manually: `xcrun appintents`-style trigger isn't documented, but opening Shortcuts.app and searching "stint" usually triggers a wake.

- [ ] **Step 4: Stop the timer + verify update propagates**

```bash
/Applications/Stint.app/Contents/MacOS/stint stop
```

Then mutate description via the GUI (or via `stint update`):

```bash
# Get the local UUID of the last entry
LAST=$(/Applications/Stint.app/Contents/MacOS/stint list --limit 1 --json | jq -r '.[0].local_uuid')
/Applications/Stint.app/Contents/MacOS/stint update "$LAST" --description "phase-c-smoke-updated"
```

Within ~10s, Spotlight search for `phase-c-smoke-updated` should return the entry.

- [ ] **Step 5: Commit verification marker**

```bash
git commit --allow-empty -m "test(6d): Phase C — cross-process Spotlight reindex works

Mutating an entry in stint-app writes a marker to ~/Library/Group
Containers/group.tech.reyem.stint/pending-reindex.json and posts the
tech.reyem.stint.reindex Darwin notification. StintIntentsExtension
wakes on the notification, drains the marker, and updates Spotlight's
index. Verified end-to-end: new entry surfaces in Spotlight within
~10 seconds of creation."
```

---

# Phase D — Retire the legacy framework

Goal: delete `swift/StintIntents/` and `swift/StintWidget/` SPM packages, remove the framework build path from `build.rs`, remove the `init_stint_intents()` dlsym scaffolding from `main.rs`, and update CI + docs accordingly. End state: only the xcodegen path produces Swift artifacts.

---

## Task D1: Remove init_stint_intents dlsym scaffolding from main.rs

**Files:**
- Modify: `crates/stint-app/src/main.rs`

- [ ] **Step 1: Remove the call site in setup()**

Open `crates/stint-app/src/main.rs`. Find the block containing `init_stint_intents();` (around line 104 of the current code, inside the setup closure). Delete that line AND the preceding comment block (the multi-line `//` comment about "the framework exports stint_intents_init...").

- [ ] **Step 2: Remove the function definition**

In the same file, find `fn init_stint_intents()` (the function itself, with its doc comment). Delete the entire function. Search for any other references to `init_stint_intents`; there should be none after Step 1.

- [ ] **Step 3: Verify build still works**

```bash
cargo build -p stint-app 2>&1 | tail -5
```

Expected: `Finished` with no errors. The framework appex is still being built by build.rs (we haven't touched that yet); it's just not initialized at startup.

- [ ] **Step 4: Commit**

```bash
git add crates/stint-app/src/main.rs
git commit -m "refactor(6d): remove init_stint_intents dlsym scaffolding from main.rs

Phase 6d's IPC path doesn't need a framework init; the App Intents
Extension self-bootstraps via @main. Spotlight indexing IPC runs through
spotlight_ipc / stint-core::ffi::notify_indexer (Darwin notification
path) instead of the dlsym-into-framework call this used to perform."
```

---

## Task D2: Remove build_stint_intents_framework from build.rs

**Files:**
- Modify: `crates/stint-app/build.rs`

- [ ] **Step 1: Remove the function and its call site**

Open `crates/stint-app/build.rs`. Find the `fn build_stint_intents_framework()` function with its long doc comment block. Delete the entire function (everything from the `///` block through the closing `}`).

Then in `main()`, delete the line:

```rust
    if let Err(e) = build_stint_intents_framework() {
        println!("cargo:warning=StintIntents framework build skipped: {e}");
    }
```

Final `main()` should be:

```rust
fn main() {
    if let Err(e) = build_stint_widget() {
        println!("cargo:warning=StintWidget appex build skipped: {e}");
    }
    if let Err(e) = build_stint_intents_extension() {
        println!("cargo:warning=StintIntentsExtension appex build skipped: {e}");
    }
    tauri_build::build()
}
```

- [ ] **Step 2: Remove now-dead helper functions if unused**

`patch_info_plist()` was only called by `build_stint_intents_framework()`. Search:

```bash
grep -c "patch_info_plist" crates/stint-app/build.rs
```

If the count is 1 (just the definition, no call site), delete the function.

`copy_dir()` and `codesign_adhoc()` are still used by `build_stint_widget()` and `build_stint_intents_extension()`. Leave them.

- [ ] **Step 3: Build to verify**

```bash
cargo build -p stint-app 2>&1 | tail -5
```

Expected: `Finished`. Two appex builds run (widget + intents extension). No framework build runs.

- [ ] **Step 4: Verify the framework is no longer rebuilt**

```bash
ls crates/stint-app/Frameworks/ 2>/dev/null
```

The directory may still exist with a stale `StintIntents.framework/` from a previous build — that's fine; nothing references it.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-app/build.rs
git commit -m "build(6d): remove build_stint_intents_framework from build.rs"
```

---

## Task D3: Remove StintIntents.framework from tauri.conf.json bundle

**Files:**
- Modify: `crates/stint-app/tauri.conf.json`

- [ ] **Step 1: Remove the framework entry**

Open `crates/stint-app/tauri.conf.json`. Find the `bundle.macOS.frameworks` array. Remove the `"Frameworks/StintIntents.framework"` string. If that's the only entry, change the array to an empty `[]`.

Also remove the two `bundle.resources` entries that reference the framework's Metadata.appintents stencil:

```json
"Frameworks/StintIntents.framework/Versions/A/Resources/Metadata.appintents/version.json": "Metadata.appintents/version.json",
"Frameworks/StintIntents.framework/Versions/A/Resources/Metadata.appintents/extract.actionsdata": "Metadata.appintents/extract.actionsdata",
```

Keep the man-page entry and any other unrelated resources.

- [ ] **Step 2: Lint the JSON**

```bash
python3 -m json.tool crates/stint-app/tauri.conf.json >/dev/null && echo "valid JSON"
```

Expected: `valid JSON`.

- [ ] **Step 3: Rebuild + verify bundle**

```bash
scripts/build-app-with-widget.sh 2>&1 | tail -5
ls target/release/bundle/macos/Stint.app/Contents/Frameworks/ 2>&1
```

Expected: `Frameworks/` directory may not exist at all, or contains nothing related to StintIntents.

- [ ] **Step 4: Commit**

```bash
git add crates/stint-app/tauri.conf.json
git commit -m "build(6d): remove StintIntents.framework from tauri.conf.json bundle"
```

---

## Task D4: Update wrapper script to drop framework re-sign step

**Files:**
- Modify: `scripts/build-app-with-widget.sh`

- [ ] **Step 1: Remove the framework re-sign block**

Open `scripts/build-app-with-widget.sh`. Find and delete the block:

```bash
echo "==> Re-signing embedded StintIntents framework (build.rs ad-hoc only)"
codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" \
  "$APP/Contents/Frameworks/StintIntents.framework"
```

- [ ] **Step 2: Verify the wrapper still passes**

```bash
scripts/build-app-with-widget.sh 2>&1 | tail -10
```

Expected: `codesign --verify` passes; no `StintIntents.framework` mentioned in output.

- [ ] **Step 3: Commit**

```bash
git add scripts/build-app-with-widget.sh
git commit -m "build(6d): wrapper script no longer re-signs deleted StintIntents framework"
```

---

## Task D5: Update release-artifacts.yml — drop framework signing + verify

**Files:**
- Modify: `.github/workflows/release-artifacts.yml`

- [ ] **Step 1: Remove framework-specific steps**

Open `.github/workflows/release-artifacts.yml`. Find and delete:

- The entire `- name: Verify StintIntents framework + App Intents metadata` step (around line 196).
- The `codesign --force --options runtime --sign "$APPLE_SIGNING_IDENTITY" "$APP_PATH/Contents/Frameworks/StintIntents.framework"` line inside the `Sign GUI binary + .app bundle` step.
- The `codesign --verify --strict --verbose=2 "$APP_PATH/Contents/Frameworks/StintIntents.framework"` line at the bottom of the same step.

- [ ] **Step 2: Add a sign step for the new StintIntentsExtension.appex**

Find the existing `codesign … StintWidget.appex` step block (added at end of 6c). Right after it, add:

```yaml
          # Sign the App Intents Extension .appex (Phase 6d). Same
          # entitlement requirement as the widget: sandbox + App Group.
          codesign --force --options runtime --timestamp \
            --sign "$APPLE_SIGNING_IDENTITY" \
            --entitlements crates/stint-app/swift/Extensions/StintIntentsExtension/StintIntentsExtension.entitlements \
            "$APP_PATH/Contents/PlugIns/StintIntentsExtension.appex"
```

Also add a verify line at the bottom of the codesign verify block:

```yaml
          codesign --verify --strict --verbose=2 "$APP_PATH/Contents/PlugIns/StintIntentsExtension.appex"
```

- [ ] **Step 3: Add the relocation step for the new appex**

Find the `Relocate StintWidget.appex into Contents/PlugIns/` step (added in 6c). Update its run script to relocate BOTH appex bundles:

```yaml
      - name: Relocate extension .appex bundles into Contents/PlugIns/
        run: |
          for name in StintWidget StintIntentsExtension; do
            SRC="crates/stint-app/PlugIns/${name}.appex"
            DEST="$APP_PATH/Contents/PlugIns/${name}.appex"
            if [ ! -d "$SRC" ]; then echo "::error::$SRC missing"; exit 1; fi
            mkdir -p "$(dirname "$DEST")"
            rm -rf "$DEST"
            cp -R "$SRC" "$DEST"
          done
          rm -rf "$APP_PATH/Contents/Resources/PlugIns"
```

- [ ] **Step 4: Lint the YAML**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release-artifacts.yml'))" && echo "valid"
```

Expected: `valid`.

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/release-artifacts.yml
git commit -m "ci(6d): release pipeline signs both extension appex bundles, drops framework"
```

---

## Task D6: Update ci.yml — remove legacy framework + SPM widget test steps

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Delete the legacy framework + widget test steps**

Open `.github/workflows/ci.yml`. Find and delete:

```yaml
      - name: Swift test (StintIntents framework)
        working-directory: crates/stint-app/swift/StintIntents
        run: xcodebuild -scheme StintIntents -destination 'platform=macOS' -derivedDataPath ./build/derived test

      - name: Swift test (StintWidget)
        working-directory: crates/stint-app/swift/StintWidget
        run: xcodebuild -scheme StintWidget -destination 'platform=macOS' -derivedDataPath ./build/derived test
```

The `Swift test (StintExtensionsCore)` step added in Task A12 already covers both — the consolidated test target tests the framework that both extensions link.

- [ ] **Step 2: Lint the YAML**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))" && echo "valid"
```

Expected: `valid`.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci(6d): drop SPM-based Swift test steps; StintExtensionsCore covers both"
```

---

## Task D7: Delete the legacy SPM Swift packages

**Files:**
- Delete: `crates/stint-app/swift/StintIntents/` (entire directory)
- Delete: `crates/stint-app/swift/StintWidget/` (entire directory)

- [ ] **Step 1: Confirm no remaining references**

```bash
grep -rn "swift/StintIntents/\|swift/StintWidget/" \
  crates/stint-app/build.rs \
  crates/stint-app/tauri.conf.json \
  scripts/build-app-with-widget.sh \
  .github/workflows/ 2>/dev/null
```

Expected: no matches. If any reference remains, fix it before deleting.

- [ ] **Step 2: Delete both directories**

```bash
git rm -r crates/stint-app/swift/StintIntents crates/stint-app/swift/StintWidget
```

- [ ] **Step 3: Verify cargo build still succeeds**

```bash
cargo build -p stint-app 2>&1 | tail -5
```

Expected: `Finished`. Two `cargo:warning=… appex rebuilt at …` lines (widget + intents). No mentions of the deleted SPM packages.

- [ ] **Step 4: Commit**

```bash
git commit -m "chore(6d): delete legacy StintIntents + StintWidget SPM packages

Both replaced by the xcodegen-driven targets in
crates/stint-app/swift/xcodegen/project.yml. Spotlight indexing now runs
in the App Intents Extension via App Group container + Darwin
notification IPC (Phase C); the framework path is fully retired."
```

---

## Task D8: Full workspace verification

**Files:** none.

- [ ] **Step 1: Format + lint + Rust tests**

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -5
cargo test --workspace -- --test-threads=1 2>&1 | grep -E "^test result|FAILED" | tail -10
```

Expected: fmt clean, no clippy warnings, every test binary green.

- [ ] **Step 2: UI typecheck + tests**

```bash
cd ui && pnpm typecheck && pnpm vitest run 2>&1 | grep -E "Test Files|Tests " | tail -3
cd ..
```

Expected: typecheck clean; all 271+ tests pass.

- [ ] **Step 3: Swift test against StintExtensionsCore**

```bash
cd crates/stint-app/swift/xcodegen
xcodegen generate >/dev/null
xcodebuild test -scheme StintExtensionsCoreTests -destination 'platform=macOS' -derivedDataPath ./build/derived 2>&1 | tail -5
cd -
```

Expected: `** TEST SUCCEEDED **`, ~9 tests pass (5 legacy + 3 SharedContainerMarker + 1 DarwinNotification).

- [ ] **Step 4: Coverage**

```bash
scripts/coverage.sh 2>&1 | tail -15
```

Expected: green across all surfaces, each ≥ 80% lines.

- [ ] **Step 5: Commit verification marker**

```bash
git commit --allow-empty -m "test(6d): full workspace verification — fmt/clippy/tests/coverage all green"
```

---

## Task D9: Manual smoke — full spec §7 checklist

**Files:** none.

Run through every item from the spec's §7 manual smoke list:

- [ ] **Step 1: Notarized install**

```bash
scripts/build-app-with-widget.sh "Developer ID Application: Reyem Technologies Inc. (WAK5K2758P)"
APP="target/release/bundle/macos/Stint.app"
rm -f "${APP}.zip"; ditto -c -k --keepParent "$APP" "${APP}.zip"
xcrun notarytool submit "${APP}.zip" --keychain-profile "stint-notary" --wait
xcrun stapler staple "$APP"
killall stint-app 2>/dev/null; sleep 1
rm -rf /Applications/Stint.app
cp -R "$APP" /Applications/
xattr -cr /Applications/Stint.app
/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister -f /Applications/Stint.app
open /Applications/Stint.app
sleep 10
```

- [ ] **Step 2: Verify pluginkit registration**

```bash
pluginkit -m -p com.apple.widgetkit-extension | grep -i stint
pluginkit -m -p com.apple.appintents-extension | grep -i stint
```

Expected: both queries list `tech.reyem.stint.widget(1.0)` and `tech.reyem.stint.intents(1.0)` respectively.

- [ ] **Step 3: Manual checks (record output in the marker commit)**

For each item below, verify and note the result:

1. Edit Widgets gallery shows Stint with three configurations × two sizes — ✅ / ❌
2. Drop the Running Timer Small onto desktop, it renders ✅ / ❌
3. Shortcuts.app search "stint" lists actions (Start, Stop, Current, etc.) — ✅ / ❌
4. Siri: "start tracking in Stint" begins a timer — ✅ / ❌
5. System Settings → Focus → pick a focus → Add Filter → Stint filter visible with project picker — ✅ / ❌
6. Spotlight: start timer with description "phase-d-spotlight", wait 10s, ⌘-Space search → entry appears — ✅ / ❌
7. Idle detection still works (existing 6c feature, not regressed) — ✅ / ❌
8. Raycast extension still works (6c, not regressed) — ✅ / ❌

- [ ] **Step 4: Commit verification marker with results**

```bash
git commit --allow-empty -m "test(6d): Phase D manual smoke — all 8 spec §7 checks pass

[Paste the 8-item ✅/❌ list from Step 3 here.]

End-state of Phase 6d reached: both 6b-deferred (Siri/Shortcuts/Focus)
and 6c-deferred (widget gallery) surfaces are live."
```

---

## Task D10: Update docs + roadmap

**Files:**
- Modify: `README.md`
- Modify: `CLAUDE.md`
- Modify: `crates/stint-cli/skills/stint/SKILL.md`

- [ ] **Step 1: README roadmap row**

Update the 6b row (still partial today) and add a 6d row:

```
| 6b | Spotlight + App Intents + Focus filter | ✅ shipped (via 6d migration) |
| 6c | Raycast + Alfred + WidgetKit + idle detection | ✅ shipped (via 6d migration for WidgetKit) |
| 6d | Xcode-based extensions: full Siri/Shortcuts/Focus + working widget gallery | ✅ shipped |
```

- [ ] **Step 2: CLAUDE.md roadmap row**

Same updates in the table in CLAUDE.md.

- [ ] **Step 3: SKILL.md — flip "NOT YET LIVE" to live**

Open `crates/stint-cli/skills/stint/SKILL.md`. Find the App Intents bullet that says "NOT YET LIVE" and replace with:

```markdown
- **App Intents (Siri / Shortcuts.app / Focus filter)** — live as of Phase 6d. Say "start tracking in Stint" to Siri; build shortcuts in Shortcuts.app; configure System Settings → Focus → Stint to auto-set a project per focus mode.
```

Find the Widget bullet — verify it still accurately describes the gallery experience (gallery now actually works).

- [ ] **Step 4: Commit**

```bash
git add README.md CLAUDE.md crates/stint-cli/skills/stint/SKILL.md
git commit -m "docs(6d): roadmap rows + SKILL.md reflect live App Intents + Widget"
```

---

## Task D11: Tag phase-6d-complete (LOCAL ONLY)

**Files:** none.

- [ ] **Step 1: Sanity check**

```bash
git status
git log --oneline | head -10
```

Expected: clean working tree; the recent commits tell the 6d story.

- [ ] **Step 2: Tag**

```bash
git tag -a phase-6d-complete -m "Phase 6d complete — Xcode-based extensions (App Intents + Widget) live"
```

- [ ] **Step 3: STOP**

Surface to the user: "Phase 6d is complete on local branch `phase-6d`, tagged `phase-6d-complete`. Ready to push and open the PR to main?"

**DO NOT** `git push`, open a PR, force-push, or trigger any release. The user explicitly governs push/release.

---

## Self-review checklist

After writing the complete plan, look at the spec with fresh eyes and check the plan against it. This is a checklist for the planner, NOT a step for subagents.

**Spec coverage:**

- §2 Goals — Siri (Phase B, verified in B7 + D9), Shortcuts.app (B7 + D9), Focus filter (D9 step 5), Edit Widgets gallery (A11 + D9 step 1), Spotlight (C9 + D9 step 6). ✅
- §3 Architecture — file moves (A3/A6/B1/C5/D7), directory layout (matches §3 diagram). ✅
- §4 Build flow — xcodegen + xcodebuild + appex repackage (A9 / B5 / D2). ✅
- §5 IPC — SharedContainerMarker (C3), DarwinNotification (C4), App Group entitlement (C1/C2), Rust-side push_pending (C6 / C7), extension-side drain (C8). ✅
- §6 Migration order A→D — preserved literally. ✅
- §7 Tests + manual smoke — D8 + D9 cover all 8 items. ✅
- §8 CI — A12 (add) + D5/D6 (update + drop legacy). ✅
- §9 Local-dev — A1. ✅
- §10 Entitlements — A7, B2, C1, C2. ✅
- §11 Trade-offs — addressed in spec, not actionable in plan. ✅
- §12 Success criteria — D8 (coverage), D9 (manual smoke), D10 (roadmap). ✅
- §13 Out-of-scope — no out-of-scope work appears in the plan. ✅

**Placeholder scan:** every step has concrete code, exact paths, exact commands. No TBD / TODO / "implement appropriate error handling" / "similar to Task N".

**Type consistency:** `SpotlightOp` enum (Rust spotlight_ipc + Swift SharedContainerMarker) uses the same camelCase variant names (`insert`/`update`/`delete`/`projectsReplaced`/`tasksReplaced`). Marker file shape (`localUuid`, `kind`) consistent across producer (stint-core `append_pending`) and consumer (Swift `SharedContainerMarker.loadOps`).

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-05-28-stint-phase-6d-xcode-extensions.md`.**

Two execution options:

**1. Subagent-Driven (recommended)** — dispatch a fresh subagent per task, two-stage review, fast iteration. Best for this plan given the variety (Swift / Rust / build scripts / CI / manual smoke).

**2. Inline Execution** — execute tasks in this session using `superpowers:executing-plans`, batch execution with checkpoints.

**Which approach?**

**Reminder before execution starts:**
- Branch from clean `main` AFTER 6c lands — NOT from `feature/task-assignment`.
- Never push, force-push, merge to main, `--no-verify`, or `--no-gpg-sign` unless the user explicitly asks for it.
- Phase D9 (manual smoke) and D11 (tag) require user-driven actions; halt before either.
