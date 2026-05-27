# stint — Phase 6c: Power-user surfaces (spec)

Four independent macOS power-user surfaces sitting on top of the Phase 6a CLI / HTTP / URL-scheme primitives: a Raycast extension, an Alfred workflow, a WidgetKit widget, and idle detection inside `stint-app`.

- **Status:** Confirmed 2026-05-27.
- **Predecessors:**
  - Phase 6a (verbs façade, MCP, HTTP API, `stint://` URL scheme, `stint skill install`, man page) — shipped.
  - Phase 6b (Spotlight indexing + URL-scheme tap routing) — shipped foundation; Siri/Shortcuts/Focus-filter discovery deferred. See [Phase 6 spec §1.5](./2026-05-25-stint-phase-6-deeper-integration-design.md).
- **Decomposition:** Four largely-independent surfaces. Each ships in its own task slice; later surfaces don't block earlier ones.

## 1. Goal

Make stint feel "where my keystrokes already live" for three distinct user types:

- **Raycast power user** — every action ⌥-Space away, with autocomplete on projects + tasks.
- **Alfred user** — same actions, fewer keystrokes, via classic keyword + script-filter workflow.
- **Always-on dashboard user** — desktop / lock-screen widget showing the running timer or today's total without launching anything.
- **Anyone** — idle detection rescues abandoned timers ("you walked away 14 minutes ago — keep or discard?").

All four consume the existing CLI / HTTP API / URL scheme. **Zero new stint-core verbs.** Whatever's in stint-core today is the API; 6c adds adapters.

## 2. Scope

### 2.1 In scope

- **Raycast extension** at `raycast-stint/` — TypeScript, 5 commands, subprocess-to-CLI.
- **Alfred workflow** at `alfred-stint/` — bash scripts, 4 keywords, subprocess-to-CLI.
- **CLI extension** — new `stint projects list-tasks <project-id>` subcommand wrapping `verbs::list_tasks` (the verb already exists; just needs the CLI clap struct + a 5-line handler). Required by Raycast's Start Timer task picker; small enough to not warrant its own phase.
- **WidgetKit widget** at `crates/stint-app/swift/StintWidget/` — Swift Package producing `.appex`, embedded under `Stint.app/Contents/PlugIns/StintWidget.appex/`. Two sizes (small + medium). Three kinds (running timer, today total, this-week project). Per-instance configurable via `WidgetConfigurationIntent`.
- **Idle detection** inside `stint-app` — `CGEventSourceSecondsSinceLastEventType` polling. Settings-configurable threshold (default 10 min). Popover banner with Keep / Discard / Discard+restart actions. Three Tauri commands backing those buttons.

### 2.2 Out of scope

- **Raycast Store publication** — covered separately by a PR to `raycast/extensions` after the extension stabilizes locally.
- **Alfred Gallery publication** — same; ship via GitHub Releases first.
- **Widget gallery iCloud sync** — Apple handles this transparently; nothing for us.
- **Lock-screen widgets** — macOS support is sparse; no demand.
- **Idle detection on Linux/Windows** — stint is macOS-only.
- **Re-enabling Siri / Shortcuts.app discovery** — that's Phase 6b.1 (still deferred). The WidgetKit `.appex` pipeline this phase establishes does open a path for that work, but it's not in 6c.

## 3. Architecture

```
┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
│  Raycast ext     │  │  Alfred workflow │  │  WidgetKit       │  │  Idle detection  │
│  (TypeScript)    │  │  (bash scripts)  │  │  (Swift .appex)  │  │  (Rust)          │
└────────┬─────────┘  └────────┬─────────┘  └────────┬─────────┘  └────────┬─────────┘
         │                     │                     │                     │
         │   stint --json …    │                     │ HTTP /v1/*           │ in-process
         │   subprocess        │                     │ on loopback          │ tokio task
         └─────────┬───────────┘                     │ (port discovery via  │
                   ▼                                 │  api.port file)      │
            ┌─────────────────────────────────────── ┴ ────────────────────┴──┐
            │              Phase 6a / 6b primitives                            │
            │  CLI binary  ·  Loopback HTTP API  ·  stint:// URL scheme        │
            └──────────────────────────────────────────────────────────────────┘
```

### 3.1 New artifacts

```
crates/stint-app/
  swift/StintWidget/                            # NEW Swift Package
    Package.swift
    Sources/StintWidget/
      StintWidgetBundle.swift                   # @main, WidgetBundle entry
      RunningTimerWidget.swift                  # Widget declaration
      WidgetConfigIntent.swift                  # WidgetConfigurationIntent
      Provider.swift                            # TimelineProvider
      Models/                                   # DTO mirrors of HTTP shapes
        EntryDTO.swift  ProjectDTO.swift  PortDiscovery.swift
      Views/
        RunningTimerView.swift  TodayTotalView.swift  WeekProjectView.swift
  src/
    idle_detector.rs                            # NEW
    commands/idle.rs                            # NEW Tauri commands
  Cargo.toml                                    # +core-graphics dep for CGEvent

ui/src/
  components/IdleBanner.tsx                     # NEW

raycast-stint/                                  # NEW top-level dir
  package.json
  src/start-timer.tsx  stop-timer.tsx  current.tsx
      recent-entries.tsx  switch-project.tsx
  README.md
  assets/icon.png

alfred-stint/                                   # NEW top-level dir
  info.plist
  start.sh  stop.sh  current.sh  recent.sh
  icon.png
  README.md
```

### 3.2 Per-surface IPC

| Surface | Talks to stint via | Why |
|---|---|---|
| Raycast | `stint --json` subprocess | Works whether GUI runs; easiest TS integration |
| Alfred | `stint --json` subprocess | Workflow scripts are bash-native |
| Widget | HTTP `/v1/*` (loopback) | Widget process is sandboxed + short-lived; can't shell out; can't link stint-core |
| Idle | in-process Rust | Lives inside stint-app, same process as the rest of the GUI |

### 3.3 New stint-app subsystems

- **`api.port` file** — on HTTP API bind, stint-app writes the bound port to `~/Library/Application Support/stint/api.port` (plain text, one line: `49792\n`). Removed on graceful shutdown. Widget reads this on every timeline refresh.
- **Widget-presence-aware HTTP auto-enable** — at setup time, if `WidgetCenter.shared.getCurrentConfigurations` (via a small Swift FFI helper) reports ≥1 stint widget installed, and `api.enabled = false`, auto-flip it to `true` and persist. One-time onboarding flicker; no user action needed.
- **Idle worker** — tokio task in `stint-app/src/idle_detector.rs`. Polls every 60s while a timer is running. Emits `idle:detected` Tauri event when activity resumes after threshold exceeded.

### 3.4 Build pipeline

- **Raycast / Alfred** — separate repos-within-a-monorepo. CI optionally lints (eslint for Raycast; shellcheck for Alfred) but doesn't affect the main release.
- **Widget** — new Swift Package + xcodebuild step in `crates/stint-app/build.rs`. Produces `.appex` (not `.framework`). The `.appex` must end up under `Stint.app/Contents/PlugIns/StintWidget.appex/` at bundle time. Tauri 2 doesn't expose a `bundle.macOS.plugins` config equivalent, so `build.rs` copies the built `.appex` to a stable path (`crates/stint-app/PlugIns/StintWidget.appex`) AND emits the same `bundle.resources` map trick used for the App Intents stencil in 6b — mapping each file in the `.appex` to its bundle-relative path under `PlugIns/StintWidget.appex/`. Plan covers the exact bundle.resources entries (it's ~6 files: Info.plist, the binary, Resources/Assets.car, Metadata.appintents, _CodeSignature/CodeResources, and any localized strings). Fallback if that path doesn't work: a post-`cargo tauri build` script that copies the `.appex` directly into the produced bundle before signing.
- **Idle detection** — pure Rust, no build-pipeline changes beyond a `core-graphics` dependency.

## 4. Raycast extension

### 4.1 Commands

| Command | Type | Args / UI | Action |
|---|---|---|---|
| **Start Timer** | Form | Description (text, required), Project (dropdown, loaded async via `stint --json projects list`), Task (dropdown, filtered by selected project, loaded via `stint --json projects list-tasks <id>` — new CLI subcommand added in 6c, see §2.1), Billable (toggle) | `stint --json start --description … [--project …] [--task …] [--billable]` |
| **Stop Timer** | No-view | — | `stint --json stop`; toasts the stopped entry's duration |
| **Current Timer** | Detail | Description, project, elapsed (live-updated every 5s while open) | `stint --json current` polled |
| **Recent Entries** | List | Last 50 entries via `stint --json list --limit 50`. Each row's actions: Restart (calls `stint --json restart <uuid>`), Copy description, Open in Stint (`stint://entry/<uuid>`) | per-action |
| **Switch Project** | Form | Project (dropdown, required) → stop + start preserving description | `stint --json stop` → `stint --json start` |

### 4.2 Preferences (Raycast standard prefs schema)

| Pref | Default | Purpose |
|---|---|---|
| **Stint binary path** | auto-detect | Override box. Auto-detect order: `which stint` → `~/.cargo/bin/stint` → `/Applications/Stint.app/Contents/MacOS/stint`. Useful for users with stint in a non-standard location. |

### 4.3 Error model

- CLI non-zero exit → parse stderr → `showToast({ style: Failure, title: "Stint", message: <stderr first line> })`.
- "Timer already running" / similar `Invariant` errors are surfaced verbatim (the CLI's `--json` mode prints structured errors via the same envelope shape as MCP).
- Binary not found at the configured path → modal with "Open preferences" action.

### 4.4 Out of scope (in Raycast)

- Backdated entries / `start_at` arg — power users can use the CLI directly.
- Reporting / weekly summary commands — defer to a follow-up if requested.

## 5. Alfred workflow

### 5.1 Keywords

| Keyword | Type | Behavior |
|---|---|---|
| `s <description>` | Run script (with arg) | Starts a timer with the given description. Project + billable take defaults from settings if available; argument is the only required input. |
| `sstop` | Run script | Stops current timer; shows large-type duration. |
| `scur` | Script filter | One result row: the running entry (or "no active timer"). ⏎ opens Stint app via `open stint://current`. |
| `srec` | Script filter | Last 20 entries. ⏎ restarts (calls `stint --json restart <uuid>`). ⌥⏎ opens via `stint://entry/<uuid>`. |

### 5.2 Bundle metadata (`info.plist`)

- `bundleid` = `tech.reyem.stint.alfred`
- `name` = "Stint"
- `description` = "Start, stop, and inspect Stint time entries from Alfred."
- `version` matches stint app version at release time.

### 5.3 Binary discovery (matches Raycast)

Scripts honor `$STINT_BIN` env var if set in the workflow's Workflow Environment Variables. Otherwise fall back to `which stint` → `~/.cargo/bin/stint` → `/Applications/Stint.app/Contents/MacOS/stint`.

### 5.4 Out of scope (in Alfred)

- Form-style multi-field input (Alfred's UX doesn't fit it). Users wanting that reach for Raycast or the popover.
- Project / task pickers — argument-only input. Reach for Raycast for autocomplete.

## 6. WidgetKit widget

### 6.1 Widget kinds (user picks at "Add Widget" time via the config sheet)

1. **Running Timer** — primary display: large elapsed time + entry description + project name. Updates every minute while running, shows "No active timer" otherwise.
2. **Today Total** — primary: sum of today's entries. Small size: just total. Medium size: breakdown by top-3 projects.
3. **This-week Project** — total for a *specific* project this week (project chosen per widget). Small: hours number. Medium: bar chart by day.

### 6.2 Sizes supported

- `.systemSmall`
- `.systemMedium`

Skipped: `.systemLarge` (overkill), `.accessoryRectangular` / `.accessoryInline` (macOS lock-screen support sparse).

### 6.3 Configuration intent

```swift
struct WidgetConfigIntent: WidgetConfigurationIntent {
    static var title: LocalizedStringResource = "Configure Stint Widget"

    @Parameter(title: "Show")
    var kind: WidgetKind

    @Parameter(title: "Project")
    var project: ProjectEntity?   // only relevant when kind == .weekProject
}

enum WidgetKind: String, AppEnum {
    case runningTimer
    case todayTotal
    case weekProject
}
```

`WidgetConfigurationIntent` is a different code path from the deferred App Intents discovery — it runs inline in the widget gallery, not via `siriactionsd`. Apple supports it cleanly in Tauri-driven builds (verified empirically: this is the same pattern that lots of third-party widget extensions use).

### 6.4 Data source: loopback HTTP

The widget process is sandboxed (Apple's WidgetKit container) — it can't:
- Shell out to CLI.
- Link `stint_core` (different process from stint-app).
- Use `dlsym` to find shared symbols.

It **can** open TCP connections to loopback. Hence: HTTP `/v1/*`.

**Port discovery:** stint-app writes the bound port to a known file on bind:

```
~/Library/Application Support/stint/api.port    ← plain text "49792\n"
```

The widget's `Provider.timeline(for:in:)`:

1. Reads `api.port` file.
2. If absent / unreadable → returns a single placeholder timeline entry ("Stint not running") with `stint://current` as the tap deep link.
3. Otherwise hits `http://127.0.0.1:<port>/v1/current` (and `/v1/entries?since=…` for the totals kinds) with a 2s connect timeout.

### 6.5 Timeline refresh

- *Running Timer* kind: 60 timeline entries spaced 1 minute apart (covers 1h). At the 50-minute mark, the last entry has refresh policy `.atEnd` which triggers a re-fetch.
- *Today Total* kind: 1 entry valid now, refresh `.after(Date.now + 300)` (5 min).
- *Week Project* kind: same as Today Total (5-min refresh).

WidgetKit constraint: ~40 entries reliable, 96 max per timeline. Apple may collapse if too aggressive. We stay at 60 for the timer kind — fine in practice.

### 6.6 Auto-enable HTTP API

stint-app's setup hook calls a Swift helper (FFI'd from the widget package — small `@_cdecl` exposing `widget_count() -> i32`) to count currently-configured widgets. If ≥1 widget exists AND `api.enabled = false`, automatically flip the setting to true and persist. One-time onboarding step; widget data starts flowing on next refresh.

### 6.7 Deep-link tap targets

Tapping a widget runs an associated intent (or opens an URL). We use the URL pattern:

- Running Timer widget → `stint://entry/<running-uuid>` (open the entry's row in Today)
- Today Total → `stint://current`
- Week Project → `stint://project/<id>`

## 7. Idle detection

### 7.1 Mechanism

```rust
extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(
        source_state_id: i32,    // 0 = combined session state
        event_type: u32,         // u32::MAX = kCGAnyInputEventType
    ) -> f64;
}

fn idle_seconds() -> f64 {
    unsafe { CGEventSourceSecondsSinceLastEventType(0, u32::MAX) }
}
```

`core-graphics` Rust crate is a thin binding; we use `extern "C"` directly to avoid the dependency footprint.

### 7.2 State machine

The detector is a tokio task spawned from `setup()`:

```
poll every 60s
  if !timer_running: continue  // no entry to worry about
  let idle = idle_seconds()

  if idle >= threshold AND pending_idle.is_none():
      pending_idle = Some(now() - idle_seconds())  // when idleness began

  if idle < 60 AND pending_idle.is_some():
      // activity resumed
      emit "idle:detected" {
          idle_started: ISO8601(pending_idle),
          idle_seconds: now() - pending_idle,
      }
      pending_idle = None
```

### 7.3 Settings

| Key | Type | Default | Purpose |
|---|---|---|---|
| `idle.enabled` | bool | `true` | Master toggle |
| `idle.threshold_secs` | u32 | `600` (10 min) | Idle period before prompt fires |

Both editable in the Settings UI under a new "Idle detection" section.

### 7.4 Tauri commands

```rust
#[tauri::command]
async fn idle_keep() -> Result<()> {
    // No-op. Banner dismisses; entry continues unchanged.
    Ok(())
}

#[tauri::command]
async fn idle_discard(idle_started: String) -> Result<()> {
    // Stop the entry with end_at = idle_started. The idle gap is excluded.
    let store = …;
    let running = RunningTimer::new(store.clone())
        .get().await?
        .ok_or_else(|| Error::Invariant("no running timer".into()))?;
    Entries::new(store.clone()).set_end(&running.local_uuid, &idle_started).await?;
    RunningTimer::new(store).clear().await?;
    Ok(())
}

#[tauri::command]
async fn idle_split(idle_started: String) -> Result<()> {
    // Same effect as discard for the storage layer — close entry at idle_started.
    // The "restart now" UX (offering a quick-restart form) lives in the UI;
    // backend just closes the existing entry.
    idle_discard(idle_started).await
}
```

### 7.5 UI banner (`ui/src/components/IdleBanner.tsx`)

```
┌────────────────────────────────────────────────────────┐
│ ⏸ You were idle for 14 minutes                         │
│                                                         │
│ [Keep]  [Discard 14m]  [Discard + restart now]         │
└────────────────────────────────────────────────────────┘
```

Slides down within the existing popover. Listens for the `idle:detected` Tauri event. Auto-dismisses after `idle_keep` is invoked or after 5 minutes of being shown (silent snooze — assume user is now active and not interested).

"Discard + restart now" opens the popover's start form pre-filled with the previous entry's description and project.

### 7.6 Edge cases

- **Sleep / Wake** — `CGEventSourceSecondsSinceLastEventType` counts sleep as "no events". On wake the function returns a large delta; the idle event fires with the correct `idle_started`. Verified pattern; works in production for other trackers (Timing, Toggl).
- **User dismisses banner via Esc and goes idle again** — latest idle period replaces previous in `pending_idle`. User always sees the *most recent* gap.
- **Timer stops via CLI / another surface while banner is visible** — `idle_discard` returns `Invariant("no running timer")` → UI shows benign toast, banner dismisses.
- **`idle.enabled = false`** — detector task runs but skips the threshold check. Cheap (one bool read every 60s).
- **Multiple displays + lock screen** — lock screen = no input events = correctly counts as idle.
- **Multiple stint-app instances (shouldn't happen but)** — each instance runs its own detector. Last-write-wins on the entry. Race acceptable; documented in code.

## 8. Data flow walkthroughs

### 8.1 Raycast: "Start Timer" command

```
user → Raycast launchbar → ⌥-Space → "stint start" → Enter
  ↓
Raycast renders Start Timer Form
  ↓ (Form mounts)
fetchProjects() = spawn `stint --json projects list`
  ↓
user fills description + picks project → Enter
  ↓
spawn `stint --json start --description … --project <id>`
  ↓ (success)
showToast({ style: Success, title: "Tracking 'X' on Acme" })
Raycast closes
```

### 8.2 Widget: timeline refresh

```
WidgetKit asks Provider.timeline(for: config, in: context)
  ↓
read ~/Library/Application Support/stint/api.port
  ↓
GET http://127.0.0.1:<port>/v1/current  (2s timeout)
  ↓ (success)
build TimelineEntry(s) — for runningTimer, 60 entries spaced 1m apart
return Timeline(entries, policy: .atEnd)
  ↓
macOS renders entry at current time
```

If port file missing OR HTTP times out:

```
return Timeline([placeholder("Stint not running", tap → stint://current)], policy: .after(1m))
```

### 8.3 Idle: detect + recover

```
idle detector task (60s poll):
  idle = 720s (12 min), threshold = 600s
  pending_idle = now - 720s = 12:34:56

  (user moves mouse)
  idle = 3s
  pending_idle.is_some() AND idle < 60s → emit
    "idle:detected" { idle_started: "12:34:56", idle_seconds: 720 }

stint-app emits Tauri event
  ↓
ui IdleBanner.tsx receives → renders banner with 3 actions
  ↓
user clicks [Discard 12m]
  ↓
invoke('idle_discard', { idle_started: "12:34:56" })
  ↓ (Rust handler)
stop the running entry with end_at = 12:34:56
clear running_timer row
sync_queue: enqueue update op
  ↓
UI banner auto-dismisses
entries:changed event fires → Today refetches
```

## 9. Error handling

### 9.1 Raycast / Alfred

- CLI non-zero exit → parse JSON error envelope from stderr → surface via toast / large-type.
- Binary not found → toast with action "Open preferences".
- Stint app not running (HTTP-dependent commands like a hypothetical "today total" command) — N/A in 6c; all Raycast/Alfred commands use CLI which works headless.

### 9.2 Widget

- HTTP fetch fails (port file missing OR connection refused OR timeout) → render placeholder widget ("Stint not running — tap to open"). Tap fires `stint://current`.
- HTTP returns non-200 → render placeholder with error message ("Stint API error").
- Configuration intent value is `nil` for `weekProject` (user didn't pick a project) → render "Pick a project in settings" placeholder.

### 9.3 Idle detection

- `CGEventSourceSecondsSinceLastEventType` returns negative or `f64::NAN` (shouldn't happen, but) → treat as 0 (no idle).
- Tauri command `idle_discard` errors because timer already stopped → benign toast, banner dismisses anyway.
- `idle.threshold_secs` is 0 or absurdly large → clamp to [60, 86400] (1 min to 24h) at read time.

## 10. Testing strategy

| Layer | Location | Run via | Coverage tracked |
|---|---|---|---|
| Raycast unit | `raycast-stint/src/*.test.tsx` | `pnpm test` inside raycast-stint/ | Local-only; not in unified report |
| Raycast e2e | manual smoke checklist in `raycast-stint/README.md` | manual | — |
| Alfred | manual smoke checklist in `alfred-stint/README.md` | manual | — |
| Widget Swift unit | `crates/stint-app/swift/StintWidget/Tests/` | `xcodebuild test` | Local-only |
| Widget e2e | manual smoke (Widget Gallery → Add Stint Widget) | manual | — |
| Idle detector Rust | `crates/stint-app/src/idle_detector.rs` inline `#[cfg(test)] mod tests` + integration via `tests/idle_detector.rs` with mock `CGEventSourceSecondsSinceLastEventType` | `cargo test` | Yes (stint-app) |
| Idle Tauri commands | `crates/stint-app/tests/idle_commands.rs` — exercises commands against a tempdir store | `cargo test` | Yes (stint-app) |
| `api.port` file | inline tests in `stint-app/src/http/` | `cargo test` | Yes (stint-app) |

### 10.1 Coverage discipline

stint-app currently sits at 83.7%. Adding `idle_detector.rs` (~80 lines) + `commands/idle.rs` (~50 lines) + widget-presence helper (~30 lines) should land above floor without additional work. Swift Widget code is excluded from the unified report (same as StintIntents in 6b).

### 10.2 What's NOT tested

- WidgetKit's actual rendering (no headless way to render a widget in CI; Apple's templates can't run unattended).
- Real lock-screen / focus / sleep behavior of idle detection.
- Raycast Store-submission CI lint (delegated to Raycast's own pipeline).

These are manual-smoke items in the plan's "Pre-merge smoke" section.

## 11. Trade-offs and deferred work

| Decision | Trade-off | Deferred alternative |
|---|---|---|
| `.appex` widget with `WidgetConfigurationIntent` | Establishes the Xcode-pipeline pattern that 6b.1 (App Intents Extension) will inherit | Static widget — simpler, no per-instance config |
| HTTP API as widget data source | Requires GUI running; widget shows placeholder otherwise | Direct SQLite read from the widget process — Apple's WidgetKit sandbox makes this hard; the placeholder is the right UX |
| `api.port` file at fixed path | Race window: stint-app might be writing while widget is reading | mtime-based caching with retry; acceptable for plain int |
| Polling every 60s for idle | 60s granularity on idle-start timestamp | Subscribe to `CGEvent` events directly — heavier, no real benefit |
| Idle threshold default 10 min | Matches Toggl/Timing | 5 min — more aggressive but more false positives |
| Discard == Split at the storage layer | "Split" name implies a behavior we're not fully implementing yet | Pre-fill the popover start form with previous entry on "Discard + restart" — captured in §7.5; lives in UI not backend |
| Raycast distribution via local-install initially | Publishing to Raycast Store has its own review cycle; ship the extension internally first | Submit to `raycast/extensions` PR after the extension is battle-tested |
| Alfred publication on GitHub Releases first | Same logic — Alfred Gallery requires curation; ship to GitHub Releases initially | Submit to Alfred Gallery later |

## 12. Implementation order (preview)

The plan doc (`docs/superpowers/plans/2026-05-27-stint-phase-6c-power-user-surfaces.md`) sequences the work. High-level order:

1. **`api.port` file** — stint-app writes/removes on bind. Required by the widget; doesn't depend on anything else.
2. **Idle detector + Tauri commands + Settings UI** — entirely self-contained Rust + small UI. Most testable, no Apple-ecosystem surface risk.
3. **`IdleBanner.tsx` UI** — Tauri event listener + 3 actions.
4. **Raycast extension** — TypeScript, 5 commands. Fully independent.
5. **Alfred workflow** — bash scripts mirroring Raycast functionality.
6. **WidgetKit package + xcodebuild + .appex bundling** — Swift Package, `build.rs` extension to invoke xcodebuild + copy `.appex` into `Contents/PlugIns/`. Hardest task — expect iteration on the build pipeline (similar to 6b's framework wrapping).
7. **WidgetConfigurationIntent + 3 widget kinds + 2 sizes**.
8. **CI gates** — `xcodebuild test` for widget; lint for Raycast; shellcheck for Alfred.
9. **Manual smoke** — checklist exercising all 4 surfaces.
10. **Docs** — extend `SKILL.md` with the new surfaces. README.md + CLAUDE.md roadmap rows for 6c.

## 13. References

- Phase 6a CLI (consumed by Raycast/Alfred): `crates/stint-cli/src/cmd/`
- Phase 6a HTTP API (consumed by widget): `crates/stint-app/src/http/handlers.rs`
- Phase 6b URL routes (consumed by widget tap targets + Raycast Recent Entries action): `crates/stint-core/src/url_scheme.rs`
- Phase 6b spec + §1.5 deferred-state context: `docs/superpowers/specs/2026-05-25-stint-phase-6-deeper-integration-design.md`

Apple references:
- WidgetKit — [`developer.apple.com/documentation/widgetkit`](https://developer.apple.com/documentation/widgetkit)
- `WidgetConfigurationIntent` — [`developer.apple.com/documentation/appintents/widgetconfigurationintent`](https://developer.apple.com/documentation/appintents/widgetconfigurationintent)
- `CGEventSourceSecondsSinceLastEventType` — [`developer.apple.com/documentation/coregraphics/1454545-cgeventsourcesecondssincelasteve`](https://developer.apple.com/documentation/coregraphics/1454545-cgeventsourcesecondssincelasteve)

Third-party references:
- Raycast Extension docs — [`developers.raycast.com`](https://developers.raycast.com)
- Alfred Workflow Object Reference — [`alfred.app/help/workflows/`](https://www.alfredapp.com/help/workflows/)
