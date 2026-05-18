# stint Phase 2: Tauri GUI + SolidJS UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a working macOS menu-bar + main-window time tracker (`stint-app`) that uses the same SQLite database as the Phase 1 CLI, so a `stint start` in the terminal and a click in the menu bar manipulate the same timer. The GUI lives at `crates/stint-app/` (Tauri shell) with a SolidJS + Tailwind frontend at `ui/`.

**Architecture:** `stint-app` is a Tauri 2 binary that depends on `stint-core` (the same library `stint-cli` uses). Its `main.rs` configures the Tauri application, registers `#[tauri::command]` wrappers around `stint-core` calls, sets up a tray icon, and manages two windows: a borderless transient popover that anchors to the tray icon, and a regular main window for browsing/editing. Both windows render the same SolidJS bundle from `ui/dist/`, distinguishing themselves by URL hash (e.g., `#/popover` vs `#/today`).

**Live updates across surfaces:** the CLI and GUI mutate the same SQLite file. The GUI polls the `running_timer` table every 1 second while a window is open (cheap — single-row SELECT) so a CLI-driven start/stop reflects in the menu bar within ~1s.

**Tech Stack:** Tauri 2 · tauri-plugin-positioner (anchor popover to tray) · SolidJS · Vite · TypeScript · Tailwind CSS 4 · Solid Router · `@modular-forms/solid` (form validation, optional) · existing workspace deps from Phase 1.

---

## File Structure

```
stint/
├── Cargo.toml                          # workspace — add stint-app member
├── crates/
│   ├── stint-core/                     # unchanged from Phase 1
│   ├── stint-cli/                      # unchanged
│   └── stint-app/                      # NEW
│       ├── Cargo.toml
│       ├── tauri.conf.json
│       ├── build.rs
│       ├── icons/
│       │   ├── icon.icns               # 32-bit RGBA, generated from a 1024×1024 PNG
│       │   ├── icon.png                # source
│       │   ├── 32x32.png
│       │   ├── 128x128.png
│       │   ├── 128x128@2x.png
│       │   └── tray.png                # template image, 22×22 + @2x
│       ├── src/
│       │   ├── main.rs                 # Tauri builder, tray, windows, dock visibility
│       │   ├── lib.rs                  # re-exports for tests
│       │   ├── app_state.rs            # AppState struct holding stint_core::Store
│       │   ├── tray.rs                 # tray icon + click handling
│       │   ├── windows.rs              # window creation/show/hide
│       │   └── commands/
│       │       ├── mod.rs
│       │       ├── timer.rs            # start/stop/get_running/heartbeat
│       │       ├── entries.rs          # list/get/edit/delete
│       │       ├── projects.rs         # list/refresh
│       │       ├── config.rs           # solidtime config get/set/test
│       │       └── sync.rs             # drain_once
│       └── tests/
│           └── commands_smoke.rs       # tauri::test-driven smoke tests
└── ui/                                  # NEW
    ├── package.json
    ├── pnpm-lock.yaml                   # committed
    ├── tsconfig.json
    ├── vite.config.ts
    ├── tailwind.config.ts
    ├── postcss.config.js
    ├── index.html
    ├── public/
    │   └── (none yet; future favicons)
    └── src/
        ├── main.tsx                    # SolidJS entry, mounts <App />
        ├── App.tsx                     # router + layout shell
        ├── routes.ts                   # route definitions
        ├── api.ts                      # typed wrappers around Tauri `invoke()`
        ├── types.ts                    # shared TS types matching Rust DTOs
        ├── stores/
        │   ├── timer.ts                # running-timer signal + 1s polling
        │   └── settings.ts             # config signal
        ├── components/
        │   ├── TimerCard.tsx           # big timer counter + start/stop
        │   ├── EntryList.tsx           # list of past entries
        │   ├── EntryRow.tsx
        │   ├── ProjectPicker.tsx
        │   ├── Duration.tsx            # formatted hh:mm:ss
        │   ├── Toast.tsx               # ephemeral status messages
        │   └── ConfigField.tsx         # generic key/value editor
        ├── routes/
        │   ├── Popover.tsx             # menu-bar UI
        │   ├── Today.tsx               # main-window today view
        │   ├── Week.tsx                # weekly summary
        │   └── Settings.tsx            # config + connection test
        └── styles.css                  # tailwind directives + tokens
```

After Phase 2 lands the user can:

- Click the menu bar icon → popover with current timer, start/stop, recent entries
- Open the main window from the popover → today/week/settings views
- Run `stint start ...` in the terminal → the menu bar icon ticks within ~1s

---

## Cross-task setup

- **Working directory:** `/Users/mariomeyer/code/ReyemTech/apps/tet`
- **Frontend package manager:** `pnpm`. Install: `brew install pnpm` (one-time).
- **Tauri CLI:** install via `cargo install tauri-cli --version "^2.0"` (one-time).
- **Branch:** start on `phase-2` branched from `phase-1-complete`.
- **Commits:** Conventional Commits as in Phase 1 (`feat(app):`, `feat(ui):`, `chore:`, `test:`).
- **End-state check after each task:** the workspace must still `cargo check --workspace` clean. Tasks that touch `ui/` must additionally `pnpm --filter stint-ui typecheck` clean (configured in Task 3).
- **Tauri command pattern:** every `#[tauri::command]` is a thin wrapper that opens an `AppState`-held `Store` and calls into `stint-core`. No domain logic lives in `stint-app`.

---

## Tasks

### Task 1: Branch + `stint-app` crate skeleton

**Files:**
- Create: `crates/stint-app/Cargo.toml`
- Create: `crates/stint-app/src/main.rs`
- Create: `crates/stint-app/src/lib.rs`
- Modify: `Cargo.toml` (root workspace)

- [ ] **Step 1: Branch**

```bash
git checkout phase-1-complete
git checkout -b phase-2
```

- [ ] **Step 2: Write `crates/stint-app/Cargo.toml`**

```toml
[package]
name = "stint-app"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true
repository.workspace = true
build = "build.rs"

[[bin]]
name = "stint-app"
path = "src/main.rs"

[dependencies]
stint-core = { path = "../stint-core" }
tokio.workspace = true
anyhow.workspace = true
serde.workspace = true
serde_json.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
chrono.workspace = true

tauri = { version = "2.1", features = ["macos-private-api", "tray-icon"] }
tauri-plugin-positioner = "1.0"

[build-dependencies]
tauri-build = { version = "2.0", features = [] }

[dev-dependencies]
tempfile.workspace = true
```

- [ ] **Step 3: Stub `main.rs`**

```rust
fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("stint-app starting");
    // Full Tauri builder lands in Task 4.
}
```

- [ ] **Step 4: Stub `lib.rs`**

```rust
//! stint-app: GUI shell over stint-core.
//!
//! Business logic lives in `stint-core`. This crate contains only Tauri
//! commands, window management, and tray plumbing.

pub mod app_state;
pub mod commands;
pub mod tray;
pub mod windows;
```

- [ ] **Step 5: Stub each declared module**

Create each of these as a one-line `// stub`:
- `crates/stint-app/src/app_state.rs`
- `crates/stint-app/src/tray.rs`
- `crates/stint-app/src/windows.rs`
- `crates/stint-app/src/commands/mod.rs`
- `crates/stint-app/src/commands/timer.rs`
- `crates/stint-app/src/commands/entries.rs`
- `crates/stint-app/src/commands/projects.rs`
- `crates/stint-app/src/commands/config.rs`
- `crates/stint-app/src/commands/sync.rs`

- [ ] **Step 6: Register the crate in the workspace**

Edit `Cargo.toml`'s `[workspace] members = [...]` to add `"crates/stint-app"`.

- [ ] **Step 7: Verify**

Run `cargo check -p stint-app`
Expected: `lib.rs` fails because `commands` module is declared but mod.rs is `// stub` referenced from `main.rs`... actually `lib.rs` references `pub mod commands;` and `commands/mod.rs` is `// stub`, which is valid. Should compile. If errors arise, capture and ask.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml crates/stint-app
git commit -m "chore(app): scaffold stint-app crate skeleton"
```

---

### Task 2: Tauri config + build script

**Files:**
- Create: `crates/stint-app/build.rs`
- Create: `crates/stint-app/tauri.conf.json`
- Create: `crates/stint-app/icons/.gitkeep` (placeholder; real icons in Task 23)

- [ ] **Step 1: `build.rs`**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 2: `tauri.conf.json`**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Stint",
  "version": "0.1.0",
  "identifier": "tech.reyem.stint",
  "build": {
    "beforeDevCommand": "pnpm --filter stint-ui dev",
    "beforeBuildCommand": "pnpm --filter stint-ui build",
    "devUrl": "http://localhost:5173",
    "frontendDist": "../../ui/dist"
  },
  "app": {
    "macOSPrivateApi": true,
    "windows": [
      {
        "label": "main",
        "title": "Stint",
        "width": 820,
        "height": 600,
        "minWidth": 640,
        "minHeight": 480,
        "visible": false,
        "url": "/#/today"
      },
      {
        "label": "popover",
        "title": "",
        "width": 320,
        "height": 380,
        "decorations": false,
        "resizable": false,
        "alwaysOnTop": true,
        "skipTaskbar": true,
        "visible": false,
        "transparent": false,
        "url": "/#/popover"
      }
    ],
    "security": {
      "csp": "default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost"
    }
  },
  "bundle": {
    "active": true,
    "targets": ["dmg", "app"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns"
    ],
    "macOS": {
      "minimumSystemVersion": "12.0",
      "category": "public.app-category.productivity"
    }
  }
}
```

- [ ] **Step 3: Create `icons/.gitkeep`** (so the directory is tracked even though real icons land in Task 23).

- [ ] **Step 4: Verify**

Run `cargo check -p stint-app`
Expected: `tauri_build` may complain about missing icons. If so, generate a placeholder 32×32 transparent PNG and copy it as `32x32.png`, `128x128.png`, `128x128@2x.png`, and convert to `icon.icns` (use `iconutil` from a `.iconset` folder, or accept the warning if `tauri_build` allows it). If `tauri_build` hard-fails, ask before proceeding — Task 23 was supposed to handle final icon assets.

- [ ] **Step 5: Commit**

```bash
git add crates/stint-app
git commit -m "chore(app): tauri config + build script"
```

---

### Task 3: SolidJS + Vite + Tailwind 4 frontend setup

**Files:**
- Create: `ui/package.json`
- Create: `ui/tsconfig.json`
- Create: `ui/vite.config.ts`
- Create: `ui/tailwind.config.ts`
- Create: `ui/postcss.config.js`
- Create: `ui/index.html`
- Create: `ui/src/main.tsx`
- Create: `ui/src/styles.css`

- [ ] **Step 1: Initialize**

Run from workspace root:
```bash
mkdir -p ui/src
cd ui
pnpm init
```

Then edit `package.json` to this:

```json
{
  "name": "stint-ui",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc --noEmit && vite build",
    "preview": "vite preview",
    "typecheck": "tsc --noEmit"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.1",
    "@tauri-apps/plugin-positioner": "^1.0",
    "@solidjs/router": "^0.15",
    "solid-js": "^1.9"
  },
  "devDependencies": {
    "@tailwindcss/postcss": "^4.0",
    "autoprefixer": "^10.4",
    "postcss": "^8.4",
    "tailwindcss": "^4.0",
    "typescript": "^5.6",
    "vite": "^5.4",
    "vite-plugin-solid": "^2.10"
  }
}
```

- [ ] **Step 2: `tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "jsx": "preserve",
    "jsxImportSource": "solid-js",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noImplicitReturns": true,
    "isolatedModules": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "types": ["vite/client"],
    "paths": {
      "~/*": ["./src/*"]
    }
  },
  "include": ["src", "vite.config.ts", "tailwind.config.ts"]
}
```

- [ ] **Step 3: `vite.config.ts`**

```ts
import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

export default defineConfig({
  plugins: [solid()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    outDir: "dist",
    target: "esnext",
    sourcemap: process.env.TAURI_ENV_DEBUG ? "inline" : false,
  },
});
```

- [ ] **Step 4: `tailwind.config.ts` + `postcss.config.js`**

`tailwind.config.ts`:
```ts
import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: ["-apple-system", "BlinkMacSystemFont", "SF Pro Text", "sans-serif"],
        mono: ["SF Mono", "Menlo", "monospace"],
      },
    },
  },
} satisfies Config;
```

`postcss.config.js`:
```js
export default {
  plugins: {
    "@tailwindcss/postcss": {},
    autoprefixer: {},
  },
};
```

- [ ] **Step 5: `index.html`**

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Stint</title>
  </head>
  <body class="bg-zinc-50 text-zinc-900 antialiased dark:bg-zinc-950 dark:text-zinc-50">
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 6: `src/styles.css`**

```css
@import "tailwindcss";

:root {
  color-scheme: light dark;
}

body {
  margin: 0;
  user-select: none;
}
```

- [ ] **Step 7: `src/main.tsx`** (Hello world only — App lands in Task 4)

```tsx
import { render } from "solid-js/web";
import "./styles.css";

function App() {
  return (
    <div class="p-4 font-sans text-sm">
      <h1 class="text-lg font-semibold">Stint is wiring up...</h1>
      <p class="text-zinc-500">Phase 2 in progress.</p>
    </div>
  );
}

render(() => <App />, document.getElementById("root")!);
```

- [ ] **Step 8: Install + typecheck + dev preview**

```bash
cd ui
pnpm install
pnpm typecheck
pnpm dev   # ctrl+c after confirming http://localhost:5173 renders
```

Expected: dev server starts, page renders the "wiring up" headline. `pnpm typecheck` exits 0.

- [ ] **Step 9: Commit**

```bash
git add ui/
git commit -m "chore(ui): scaffold SolidJS + Vite + Tailwind 4"
```

---

### Task 4: Wire Tauri to load the SolidJS bundle (Hello world end-to-end)

**Files:**
- Modify: `crates/stint-app/src/main.rs`
- Modify: `crates/stint-app/src/app_state.rs`
- Modify: `crates/stint-app/src/windows.rs`
- Modify: `ui/src/main.tsx`

- [ ] **Step 1: `AppState`** — holds the shared `Store`.

`crates/stint-app/src/app_state.rs`:
```rust
use std::sync::Arc;
use stint_core::store::Store;

pub struct AppState {
    pub store: Arc<Store>,
}

impl AppState {
    pub async fn init() -> anyhow::Result<Self> {
        stint_core::paths::ensure_data_dir()?;
        let db_path = stint_core::paths::database_path()?;
        let store = Store::connect(&db_path).await?;
        Ok(Self { store: Arc::new(store) })
    }
}
```

- [ ] **Step 2: Window helpers**

`crates/stint-app/src/windows.rs`:
```rust
use tauri::{AppHandle, Manager, WebviewWindow};

pub fn show_main(app: &AppHandle) -> tauri::Result<()> {
    if let Some(win) = app.get_webview_window("main") {
        win.show()?;
        win.set_focus()?;
    }
    Ok(())
}

pub fn show_popover(app: &AppHandle) -> tauri::Result<()> {
    if let Some(win) = app.get_webview_window("popover") {
        win.show()?;
        win.set_focus()?;
    }
    Ok(())
}

pub fn hide_popover(app: &AppHandle) -> tauri::Result<()> {
    if let Some(win) = app.get_webview_window("popover") {
        win.hide()?;
    }
    Ok(())
}

pub fn focus_or_show_main(app: &AppHandle) -> tauri::Result<()> {
    show_main(app)
}

pub fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("main")
}
```

- [ ] **Step 3: `main.rs`**

```rust
mod app_state;
mod commands;
mod tray;
mod windows;

use anyhow::Result;
use app_state::AppState;
use tauri::Manager;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("STINT_LOG")
                .unwrap_or_else(|_| "info".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let app_state = AppState::init().await?;

    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .manage(RwLock::new(app_state))
        .setup(|app| {
            // Show main window for now; tray + popover wiring lands in Tasks 16-18.
            windows::show_main(&app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())?;

    Ok(())
}
```

- [ ] **Step 4: SolidJS — render something that proves IPC works**

`ui/src/main.tsx`:
```tsx
import { render } from "solid-js/web";
import { createSignal, onMount } from "solid-js";
import { getVersion } from "@tauri-apps/api/app";
import "./styles.css";

function App() {
  const [version, setVersion] = createSignal<string>("…");

  onMount(async () => {
    try {
      setVersion(await getVersion());
    } catch (e) {
      setVersion(`ipc error: ${(e as Error).message}`);
    }
  });

  return (
    <div class="p-6 font-sans">
      <h1 class="text-xl font-semibold">Stint</h1>
      <p class="mt-2 text-sm text-zinc-500">App version: {version()}</p>
    </div>
  );
}

render(() => <App />, document.getElementById("root")!);
```

- [ ] **Step 5: Run**

```bash
cd crates/stint-app
cargo tauri dev
```

Expected: main window opens, shows "App version: 0.1.0". Frontend reload works on save.

- [ ] **Step 6: Commit**

```bash
git add crates/stint-app ui/
git commit -m "feat(app): hello-world end-to-end Tauri ↔ SolidJS"
```

---

### Task 5: Tauri command infrastructure + first command

**Files:**
- Modify: `crates/stint-app/src/commands/mod.rs`
- Modify: `crates/stint-app/src/commands/timer.rs`
- Modify: `crates/stint-app/src/main.rs`

This task introduces the pattern every later command follows.

- [ ] **Step 1: Command module dispatch**

`crates/stint-app/src/commands/mod.rs`:
```rust
pub mod config;
pub mod entries;
pub mod projects;
pub mod sync;
pub mod timer;

use crate::app_state::AppState;
use std::sync::Arc;
use stint_core::store::Store;
use tauri::State;
use tokio::sync::RwLock;

pub(crate) async fn store(state: &State<'_, RwLock<AppState>>) -> Arc<Store> {
    state.read().await.store.clone()
}

#[derive(Debug, serde::Serialize)]
pub struct AppError {
    pub kind: String,
    pub message: String,
}

impl From<stint_core::Error> for AppError {
    fn from(e: stint_core::Error) -> Self {
        Self {
            kind: format!("{:?}", std::mem::discriminant(&e)),
            message: e.to_string(),
        }
    }
}
```

- [ ] **Step 2: First command — `get_running_timer`**

`crates/stint-app/src/commands/timer.rs`:
```rust
use crate::app_state::AppState;
use crate::commands::{store, AppError};
use serde::Serialize;
use stint_core::store::entries::Entries;
use stint_core::store::running::RunningTimer;
use tauri::State;
use tokio::sync::RwLock;

#[derive(Serialize)]
pub struct RunningTimerView {
    pub local_uuid: String,
    pub description: String,
    pub start_at: String,
    pub project_id: Option<String>,
}

#[tauri::command]
pub async fn get_running_timer(
    state: State<'_, RwLock<AppState>>,
) -> Result<Option<RunningTimerView>, AppError> {
    let store = store(&state).await;
    let running = RunningTimer::new((*store).clone());
    let Some(r) = running.get().await? else {
        return Ok(None);
    };
    let entries = Entries::new((*store).clone());
    let entry = entries.get(&r.local_uuid).await?;
    Ok(entry.map(|e| RunningTimerView {
        local_uuid: e.local_uuid,
        description: e.description,
        start_at: e.start_at,
        project_id: e.project_id,
    }))
}
```

- [ ] **Step 3: Register the command in `main.rs`**

In the `tauri::Builder::default()` chain, add:
```rust
.invoke_handler(tauri::generate_handler![
    commands::timer::get_running_timer,
])
```

(Add it just before `.setup(...)`.)

- [ ] **Step 4: Smoke-test from the UI**

Replace `ui/src/main.tsx`:
```tsx
import { render } from "solid-js/web";
import { createSignal, onMount } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

type RunningTimer = {
  local_uuid: string;
  description: string;
  start_at: string;
  project_id: string | null;
};

function App() {
  const [timer, setTimer] = createSignal<RunningTimer | null>(null);

  onMount(async () => {
    const t = await invoke<RunningTimer | null>("get_running_timer");
    setTimer(t);
  });

  return (
    <div class="p-6 font-sans text-sm">
      <h1 class="text-lg font-semibold">Stint</h1>
      {timer() ? (
        <p class="mt-2">Running: {timer()!.description}</p>
      ) : (
        <p class="mt-2 text-zinc-500">No timer running.</p>
      )}
    </div>
  );
}

render(() => <App />, document.getElementById("root")!);
```

- [ ] **Step 5: Run and verify**

```bash
# Set a running timer via CLI first
stint start "ipc smoke test"

# In another shell:
cargo tauri dev -p stint-app
```

Expected: window opens, shows "Running: ipc smoke test".

- [ ] **Step 6: Commit**

```bash
git add crates/stint-app ui/
git commit -m "feat(app): first tauri command — get_running_timer"
```

---

### Task 6-10: Remaining Tauri commands (one task per file)

The pattern from Task 5 repeats. For brevity, each task below specifies the file, the command signatures, and the registration step. Each command is a thin wrapper over `stint-core` types we already built in Phase 1.

### Task 6: Timer commands — start, stop, delete, update_description

**File:** `crates/stint-app/src/commands/timer.rs`

Append to the file:

```rust
use stint_core::timer::{StartArgs, TimerService};

#[derive(serde::Deserialize)]
pub struct StartTimerArgs {
    pub description: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
}

#[tauri::command]
pub async fn start_timer(
    state: State<'_, RwLock<AppState>>,
    args: StartTimerArgs,
) -> Result<String, AppError> {
    let store = store(&state).await;
    let timer = TimerService::new((*store).clone());
    let id = timer
        .start(StartArgs {
            description: args.description,
            project_id: args.project_id,
            task_id: args.task_id,
            source: "gui".into(),
        })
        .await?;
    Ok(id)
}

#[tauri::command]
pub async fn stop_timer(
    state: State<'_, RwLock<AppState>>,
) -> Result<String, AppError> {
    let store = store(&state).await;
    let timer = TimerService::new((*store).clone());
    Ok(timer.stop().await?)
}

#[tauri::command]
pub async fn delete_entry(
    state: State<'_, RwLock<AppState>>,
    local_uuid: String,
) -> Result<(), AppError> {
    let store = store(&state).await;
    let timer = TimerService::new((*store).clone());
    timer.delete(&local_uuid).await?;
    Ok(())
}

#[tauri::command]
pub async fn update_description(
    state: State<'_, RwLock<AppState>>,
    local_uuid: String,
    description: String,
) -> Result<(), AppError> {
    let store = store(&state).await;
    let timer = TimerService::new((*store).clone());
    timer.update_description(&local_uuid, &description).await?;
    Ok(())
}
```

Register all four in `main.rs`'s `generate_handler!` list.

Commit: `feat(app): timer commands (start/stop/delete/update)`

---

### Task 7: Entry listing commands — list_today, list_between

**File:** `crates/stint-app/src/commands/entries.rs`

```rust
use crate::app_state::AppState;
use crate::commands::{store, AppError};
use chrono::{Local, TimeZone, Utc};
use serde::Serialize;
use stint_core::store::entries::{Entries, TimeEntryRow};
use tauri::State;
use tokio::sync::RwLock;

#[derive(Serialize)]
pub struct EntryView {
    pub local_uuid: String,
    pub solidtime_id: Option<String>,
    pub description: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub start_at: String,
    pub end_at: Option<String>,
    pub sync_state: String,
    pub source: String,
}

impl From<TimeEntryRow> for EntryView {
    fn from(r: TimeEntryRow) -> Self {
        Self {
            local_uuid: r.local_uuid,
            solidtime_id: r.solidtime_id,
            description: r.description,
            project_id: r.project_id,
            task_id: r.task_id,
            start_at: r.start_at,
            end_at: r.end_at,
            sync_state: r.sync_state,
            source: r.source,
        }
    }
}

#[tauri::command]
pub async fn list_today(
    state: State<'_, RwLock<AppState>>,
) -> Result<Vec<EntryView>, AppError> {
    let store = store(&state).await;
    let today = Local::now().date_naive();
    let start_local = Local
        .from_local_datetime(&today.and_hms_opt(0, 0, 0).unwrap())
        .unwrap();
    let end_local = start_local + chrono::Duration::days(1);
    let from = start_local.with_timezone(&Utc).to_rfc3339();
    let to = end_local.with_timezone(&Utc).to_rfc3339();

    let entries = Entries::new((*store).clone());
    let rows = entries.list_between(&from, &to).await?;
    Ok(rows.into_iter().map(EntryView::from).collect())
}

#[tauri::command]
pub async fn list_between(
    state: State<'_, RwLock<AppState>>,
    from: String,
    to: String,
) -> Result<Vec<EntryView>, AppError> {
    let store = store(&state).await;
    let entries = Entries::new((*store).clone());
    let rows = entries.list_between(&from, &to).await?;
    Ok(rows.into_iter().map(EntryView::from).collect())
}
```

Register both. Commit: `feat(app): entry list commands`.

---

### Task 8: Project commands — list_projects, refresh_projects

**File:** `crates/stint-app/src/commands/projects.rs`

```rust
use crate::app_state::AppState;
use crate::commands::{store, AppError};
use anyhow::anyhow;
use stint_core::config::{secrets::Secrets, Settings};
use stint_core::solidtime::SolidtimeClient;
use stint_core::store::reference::{ProjectRow, Reference};
use stint_core::sync::refresh::refresh_reference_data;
use tauri::State;
use tokio::sync::RwLock;

async fn build_client(
    store: &stint_core::store::Store,
) -> Result<SolidtimeClient, AppError> {
    let settings = Settings::new(store.clone());
    let secrets = Secrets::default();
    let url = settings
        .get("solidtime.url")
        .await?
        .ok_or_else(|| stint_core::Error::MissingConfig("solidtime.url"))?;
    let token = secrets
        .get("solidtime.token")?
        .ok_or_else(|| stint_core::Error::MissingConfig("solidtime.token"))?;
    let org = settings
        .get("solidtime.org")
        .await?
        .ok_or_else(|| stint_core::Error::MissingConfig("solidtime.org"))?;
    Ok(SolidtimeClient::new(&url, &token).with_org(org))
}

#[tauri::command]
pub async fn list_projects(
    state: State<'_, RwLock<AppState>>,
) -> Result<Vec<ProjectRow>, AppError> {
    let store = store(&state).await;
    let r = Reference::new((*store).clone());
    Ok(r.list_projects().await?)
}

#[tauri::command]
pub async fn refresh_projects(
    state: State<'_, RwLock<AppState>>,
) -> Result<usize, AppError> {
    let store = store(&state).await;
    let client = build_client(&store).await?;
    refresh_reference_data(&store, &client).await?;
    let r = Reference::new((*store).clone());
    Ok(r.list_projects().await?.len())
}
```

(`anyhow` import unused — drop it.) Register both. Commit: `feat(app): project list + refresh commands`.

---

### Task 9: Config commands — get/set/test

**File:** `crates/stint-app/src/commands/config.rs`

```rust
use crate::app_state::AppState;
use crate::commands::{store, AppError};
use serde::Serialize;
use stint_core::config::{secrets::Secrets, Settings};
use stint_core::solidtime::SolidtimeClient;
use tauri::State;
use tokio::sync::RwLock;

const SECRET_KEYS: &[&str] = &["solidtime.token"];

#[derive(Serialize)]
pub struct ConfigView {
    pub key: String,
    pub value: Option<String>,
    pub is_secret: bool,
    pub present: bool,
}

#[tauri::command]
pub async fn config_show(
    state: State<'_, RwLock<AppState>>,
) -> Result<Vec<ConfigView>, AppError> {
    let store = store(&state).await;
    let settings = Settings::new((*store).clone());
    let secrets = Secrets::default();

    let mut out: Vec<ConfigView> = settings
        .list_prefixed("")
        .await?
        .into_iter()
        .map(|(k, v)| ConfigView {
            key: k,
            value: Some(v),
            is_secret: false,
            present: true,
        })
        .collect();

    for k in SECRET_KEYS {
        let present = secrets.get(k)?.is_some();
        out.push(ConfigView {
            key: (*k).to_string(),
            value: None,
            is_secret: true,
            present,
        });
    }
    Ok(out)
}

#[tauri::command]
pub async fn config_set(
    state: State<'_, RwLock<AppState>>,
    key: String,
    value: String,
) -> Result<(), AppError> {
    let store = store(&state).await;
    if SECRET_KEYS.contains(&key.as_str()) {
        Secrets::default().set(&key, &value)?;
    } else {
        Settings::new((*store).clone()).set(&key, &value).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn config_test(
    state: State<'_, RwLock<AppState>>,
) -> Result<String, AppError> {
    let store = store(&state).await;
    let settings = Settings::new((*store).clone());
    let url = settings
        .get("solidtime.url")
        .await?
        .ok_or_else(|| stint_core::Error::MissingConfig("solidtime.url"))?;
    let token = Secrets::default()
        .get("solidtime.token")?
        .ok_or_else(|| stint_core::Error::MissingConfig("solidtime.token"))?;
    let client = SolidtimeClient::new(&url, &token);
    let me = client.test_connection().await?;
    Ok(me.email.unwrap_or(me.id))
}
```

Register all three. Commit: `feat(app): config commands`.

---

### Task 10: Sync command — drain_once

**File:** `crates/stint-app/src/commands/sync.rs`

```rust
use crate::app_state::AppState;
use crate::commands::{store, AppError};
use stint_core::config::{secrets::Secrets, Settings};
use stint_core::solidtime::SolidtimeClient;
use stint_core::sync::drain_once;
use tauri::State;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn sync_now(
    state: State<'_, RwLock<AppState>>,
) -> Result<usize, AppError> {
    let store = store(&state).await;
    let settings = Settings::new((*store).clone());
    let url = settings
        .get("solidtime.url")
        .await?
        .ok_or_else(|| stint_core::Error::MissingConfig("solidtime.url"))?;
    let token = Secrets::default()
        .get("solidtime.token")?
        .ok_or_else(|| stint_core::Error::MissingConfig("solidtime.token"))?;
    let org = settings
        .get("solidtime.org")
        .await?
        .ok_or_else(|| stint_core::Error::MissingConfig("solidtime.org"))?;
    let client = SolidtimeClient::new(&url, &token).with_org(org);
    Ok(drain_once(&store, &client).await?)
}
```

Register. Commit: `feat(app): sync_now command`.

---

### Task 11: Tauri command smoke test

**File:** `crates/stint-app/tests/commands_smoke.rs`

Tauri 2's testing utilities create a mock `App`. The pattern:

```rust
use std::sync::Arc;
use stint_app::app_state::AppState;
use stint_core::store::Store;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tempfile::TempDir;
use tokio::sync::RwLock;

async fn build_test_app() -> (tauri::App<tauri::test::MockRuntime>, TempDir) {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    let store = Store::connect(&db).await.unwrap();
    let state = AppState { store: Arc::new(store) };

    let app = mock_builder()
        .manage(RwLock::new(state))
        .invoke_handler(tauri::generate_handler![
            stint_app::commands::timer::get_running_timer,
            stint_app::commands::timer::start_timer,
            stint_app::commands::timer::stop_timer,
        ])
        .build(mock_context(noop_assets()))
        .unwrap();
    (app, tmp)
}

#[tokio::test]
async fn get_running_timer_returns_none_initially() {
    let (app, _tmp) = build_test_app().await;
    let webview = tauri::WebviewWindowBuilder::new(&app, "test", tauri::WebviewUrl::default())
        .build()
        .unwrap();

    let result: tauri::ipc::InvokeResponseBody = tauri::test::get_ipc_response(
        &webview,
        tauri::test::INVOKE_ON("get_running_timer", serde_json::json!({})),
    )
    .unwrap();
    let v: serde_json::Value = result.try_into().unwrap();
    assert!(v.is_null());
}
```

If the Tauri test API differs at runtime, simplify to a build-only smoke test that just verifies the `generate_handler!` macro expansion succeeds.

Commit: `test(app): commands smoke test`.

---

### Task 12: SolidJS API binding layer

**Files:**
- Create: `ui/src/types.ts`
- Create: `ui/src/api.ts`

`ui/src/types.ts`:
```ts
export type RunningTimer = {
  local_uuid: string;
  description: string;
  start_at: string;
  project_id: string | null;
};

export type Entry = {
  local_uuid: string;
  solidtime_id: string | null;
  description: string;
  project_id: string | null;
  task_id: string | null;
  start_at: string;
  end_at: string | null;
  sync_state: "synced" | "dirty" | "pending_create" | "pending_delete";
  source: string;
};

export type Project = {
  id: string;
  name: string;
  color: string | null;
  client_id: string | null;
  archived: number;
};

export type ConfigEntry = {
  key: string;
  value: string | null;
  is_secret: boolean;
  present: boolean;
};

export type AppError = {
  kind: string;
  message: string;
};
```

`ui/src/api.ts`:
```ts
import { invoke } from "@tauri-apps/api/core";
import type { Entry, Project, RunningTimer, ConfigEntry } from "./types";

export const api = {
  // timer
  getRunningTimer: () => invoke<RunningTimer | null>("get_running_timer"),
  startTimer: (description: string, projectId?: string, taskId?: string) =>
    invoke<string>("start_timer", { args: { description, project_id: projectId ?? null, task_id: taskId ?? null } }),
  stopTimer: () => invoke<string>("stop_timer"),
  deleteEntry: (localUuid: string) =>
    invoke<void>("delete_entry", { localUuid }),
  updateDescription: (localUuid: string, description: string) =>
    invoke<void>("update_description", { localUuid, description }),

  // entries
  listToday: () => invoke<Entry[]>("list_today"),
  listBetween: (from: string, to: string) =>
    invoke<Entry[]>("list_between", { from, to }),

  // projects
  listProjects: () => invoke<Project[]>("list_projects"),
  refreshProjects: () => invoke<number>("refresh_projects"),

  // config
  configShow: () => invoke<ConfigEntry[]>("config_show"),
  configSet: (key: string, value: string) =>
    invoke<void>("config_set", { key, value }),
  configTest: () => invoke<string>("config_test"),

  // sync
  syncNow: () => invoke<number>("sync_now"),
};
```

Commit: `feat(ui): typed API bindings`.

---

### Task 13: Timer store (signal + 1s polling)

**File:** `ui/src/stores/timer.ts`

```ts
import { createSignal, onCleanup } from "solid-js";
import { api } from "~/api";
import type { RunningTimer } from "~/types";

export function useTimerStore() {
  const [running, setRunning] = createSignal<RunningTimer | null>(null);
  const [elapsedSecs, setElapsedSecs] = createSignal(0);

  async function refresh() {
    try {
      const t = await api.getRunningTimer();
      setRunning(t);
      if (t) {
        const startMs = new Date(t.start_at).getTime();
        setElapsedSecs(Math.floor((Date.now() - startMs) / 1000));
      } else {
        setElapsedSecs(0);
      }
    } catch (e) {
      console.warn("timer refresh failed", e);
    }
  }

  // Poll every 1s — picks up CLI-driven changes within a second.
  const id = window.setInterval(refresh, 1000);
  refresh();
  onCleanup(() => window.clearInterval(id));

  return {
    running,
    elapsedSecs,
    refresh,
    async start(description: string, projectId?: string) {
      await api.startTimer(description, projectId);
      await refresh();
    },
    async stop() {
      await api.stopTimer();
      await refresh();
    },
  };
}
```

Commit: `feat(ui): timer store with 1s polling`.

---

### Task 14: TimerCard component + Duration helper

**Files:**
- Create: `ui/src/components/Duration.tsx`
- Create: `ui/src/components/TimerCard.tsx`

`Duration.tsx`:
```tsx
export function formatDuration(secs: number): string {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = secs % 60;
  const pad = (n: number) => n.toString().padStart(2, "0");
  return `${pad(h)}:${pad(m)}:${pad(s)}`;
}

export default function Duration(props: { seconds: number }) {
  return (
    <span class="font-mono tabular-nums">{formatDuration(props.seconds)}</span>
  );
}
```

`TimerCard.tsx`:
```tsx
import { Show, createSignal } from "solid-js";
import Duration from "./Duration";
import { useTimerStore } from "~/stores/timer";

export default function TimerCard() {
  const timer = useTimerStore();
  const [description, setDescription] = createSignal("");

  return (
    <div class="rounded-xl border border-zinc-200 bg-white p-4 shadow-sm dark:border-zinc-800 dark:bg-zinc-900">
      <Show
        when={timer.running()}
        fallback={
          <div class="flex items-center gap-2">
            <input
              class="flex-1 rounded border border-zinc-300 px-2 py-1 text-sm dark:border-zinc-700 dark:bg-zinc-800"
              placeholder="What are you working on?"
              value={description()}
              onInput={(e) => setDescription(e.currentTarget.value)}
            />
            <button
              class="rounded bg-zinc-900 px-3 py-1 text-sm font-semibold text-white dark:bg-white dark:text-zinc-900"
              disabled={!description().trim()}
              onClick={() => timer.start(description().trim()).then(() => setDescription(""))}
            >
              Start
            </button>
          </div>
        }
      >
        {(t) => (
          <div>
            <div class="flex items-baseline justify-between">
              <span class="text-xs uppercase tracking-wide text-zinc-500">
                Tracking
              </span>
              <span class="text-xs text-green-600">● Live</span>
            </div>
            <div class="mt-1 text-3xl font-semibold tabular-nums">
              <Duration seconds={timer.elapsedSecs()} />
            </div>
            <div class="mt-1 text-sm text-zinc-500">{t().description}</div>
            <button
              class="mt-3 w-full rounded bg-zinc-900 py-1.5 text-sm font-semibold text-white dark:bg-white dark:text-zinc-900"
              onClick={() => timer.stop()}
            >
              Stop
            </button>
          </div>
        )}
      </Show>
    </div>
  );
}
```

Commit: `feat(ui): timer card + duration formatter`.

---

### Task 15: EntryList + EntryRow

**Files:**
- Create: `ui/src/components/EntryList.tsx`
- Create: `ui/src/components/EntryRow.tsx`

`EntryRow.tsx`:
```tsx
import Duration, { formatDuration } from "./Duration";
import type { Entry } from "~/types";

function durationSecs(start: string, end: string | null): number {
  const s = new Date(start).getTime();
  const e = end ? new Date(end).getTime() : Date.now();
  return Math.max(0, Math.floor((e - s) / 1000));
}

export default function EntryRow(props: {
  entry: Entry;
  onDelete?: (id: string) => void;
}) {
  const isRunning = !props.entry.end_at;
  return (
    <li class="flex items-center justify-between border-b border-zinc-100 py-2 dark:border-zinc-800">
      <div class="min-w-0">
        <div class="truncate text-sm">{props.entry.description}</div>
        <div class="text-xs text-zinc-500">
          <span
            class="rounded px-1 text-[10px] font-medium uppercase"
            classList={{
              "bg-zinc-100 text-zinc-600 dark:bg-zinc-800": !isRunning,
              "bg-green-100 text-green-700 dark:bg-green-950 dark:text-green-300":
                isRunning,
            }}
          >
            {isRunning ? "Running" : props.entry.sync_state}
          </span>
        </div>
      </div>
      <div class="flex items-center gap-3">
        <span class="font-mono tabular-nums text-sm">
          {formatDuration(durationSecs(props.entry.start_at, props.entry.end_at))}
        </span>
        <button
          class="text-xs text-zinc-400 hover:text-red-600"
          onClick={() => props.onDelete?.(props.entry.local_uuid)}
        >
          Delete
        </button>
      </div>
    </li>
  );
}
```

`EntryList.tsx`:
```tsx
import { For, Show } from "solid-js";
import EntryRow from "./EntryRow";
import type { Entry } from "~/types";

export default function EntryList(props: {
  entries: Entry[];
  onDelete?: (id: string) => void;
}) {
  return (
    <Show
      when={props.entries.length > 0}
      fallback={
        <p class="py-4 text-center text-sm text-zinc-500">No entries.</p>
      }
    >
      <ul class="divide-y divide-zinc-100 dark:divide-zinc-800">
        <For each={props.entries}>
          {(e) => <EntryRow entry={e} onDelete={props.onDelete} />}
        </For>
      </ul>
    </Show>
  );
}
```

Commit: `feat(ui): entry list + row`.

---

### Task 16: Routes + App shell

**Files:**
- Create: `ui/src/App.tsx`
- Create: `ui/src/routes/Popover.tsx`
- Create: `ui/src/routes/Today.tsx`
- Create: `ui/src/routes/Settings.tsx`
- Modify: `ui/src/main.tsx`

`App.tsx`:
```tsx
import { Router, Route } from "@solidjs/router";
import Popover from "./routes/Popover";
import Today from "./routes/Today";
import Settings from "./routes/Settings";

export default function App() {
  return (
    <Router>
      <Route path="/popover" component={Popover} />
      <Route path="/today" component={Today} />
      <Route path="/settings" component={Settings} />
    </Router>
  );
}
```

`main.tsx`:
```tsx
import { render } from "solid-js/web";
import App from "./App";
import "./styles.css";

render(() => <App />, document.getElementById("root")!);
```

`Popover.tsx` (compact menu-bar UI):
```tsx
import TimerCard from "~/components/TimerCard";

export default function Popover() {
  return (
    <div class="w-[300px] p-3">
      <TimerCard />
      <button
        class="mt-3 w-full rounded border border-zinc-300 py-1 text-xs text-zinc-600 dark:border-zinc-700 dark:text-zinc-300"
        onClick={() => window.location.assign("/#/today")}
      >
        Open main window
      </button>
    </div>
  );
}
```

`Today.tsx`:
```tsx
import { createResource, Show } from "solid-js";
import { api } from "~/api";
import TimerCard from "~/components/TimerCard";
import EntryList from "~/components/EntryList";

export default function Today() {
  const [entries, { refetch }] = createResource(() => api.listToday());

  return (
    <div class="mx-auto max-w-2xl p-6">
      <header class="mb-4 flex items-baseline justify-between">
        <h1 class="text-lg font-semibold">Today</h1>
        <nav class="text-xs text-zinc-500">
          <a class="mr-3 hover:underline" href="/#/today">Today</a>
          <a class="hover:underline" href="/#/settings">Settings</a>
        </nav>
      </header>

      <TimerCard />

      <section class="mt-6">
        <h2 class="mb-2 text-sm font-medium text-zinc-700 dark:text-zinc-300">
          Entries
        </h2>
        <Show
          when={!entries.loading}
          fallback={<p class="text-sm text-zinc-500">Loading…</p>}
        >
          <EntryList
            entries={entries() ?? []}
            onDelete={async (id) => {
              await api.deleteEntry(id);
              refetch();
            }}
          />
        </Show>
      </section>
    </div>
  );
}
```

`Settings.tsx`:
```tsx
import { createResource, createSignal, For, Show } from "solid-js";
import { api } from "~/api";

export default function Settings() {
  const [config, { refetch }] = createResource(() => api.configShow());
  const [status, setStatus] = createSignal<string | null>(null);

  async function setValue(key: string, value: string) {
    await api.configSet(key, value);
    setStatus(`Saved ${key}.`);
    refetch();
  }

  async function test() {
    try {
      const who = await api.configTest();
      setStatus(`✓ connected as ${who}`);
    } catch (e) {
      setStatus(`✗ ${(e as { message: string }).message}`);
    }
  }

  return (
    <div class="mx-auto max-w-2xl p-6">
      <header class="mb-4 flex items-baseline justify-between">
        <h1 class="text-lg font-semibold">Settings</h1>
        <nav class="text-xs text-zinc-500">
          <a class="mr-3 hover:underline" href="/#/today">Today</a>
          <a class="hover:underline" href="/#/settings">Settings</a>
        </nav>
      </header>

      <Show when={status()}>
        <div class="mb-3 text-xs text-zinc-500">{status()}</div>
      </Show>

      <Show when={config()}>
        <ul class="space-y-2">
          <For each={config()!}>
            {(c) => (
              <li class="flex items-center gap-2">
                <label class="w-48 text-xs text-zinc-500">{c.key}</label>
                <Show
                  when={!c.is_secret}
                  fallback={
                    <input
                      type="password"
                      placeholder={c.present ? "•••• (set)" : "(unset)"}
                      class="flex-1 rounded border border-zinc-300 px-2 py-1 text-sm dark:border-zinc-700 dark:bg-zinc-800"
                      onChange={(e) => setValue(c.key, e.currentTarget.value)}
                    />
                  }
                >
                  <input
                    value={c.value ?? ""}
                    class="flex-1 rounded border border-zinc-300 px-2 py-1 text-sm dark:border-zinc-700 dark:bg-zinc-800"
                    onChange={(e) => setValue(c.key, e.currentTarget.value)}
                  />
                </Show>
              </li>
            )}
          </For>
        </ul>
      </Show>

      <button
        class="mt-4 rounded bg-zinc-900 px-3 py-1 text-sm text-white dark:bg-white dark:text-zinc-900"
        onClick={test}
      >
        Test connection
      </button>
    </div>
  );
}
```

Commit: `feat(ui): routes — popover/today/settings`.

---

### Task 17: Tray icon

**File:** `crates/stint-app/src/tray.rs`

```rust
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use crate::windows;

pub fn build(app: &AppHandle) -> tauri::Result<TrayIcon> {
    let icon = Image::from_bytes(include_bytes!("../icons/tray.png"))
        .unwrap_or_else(|_| app.default_window_icon().cloned().unwrap());

    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "open", "Open Stint", true, None::<&str>)?,
            &MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?,
        ],
    )?;

    let tray = TrayIconBuilder::with_id("stint-tray")
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                let _ = windows::show_main(app);
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button, .. } = event {
                if matches!(button, tauri::tray::MouseButton::Left) {
                    let app = tray.app_handle();
                    // Toggle popover visibility on left-click.
                    if let Some(win) = app.get_webview_window("popover") {
                        if win.is_visible().unwrap_or(false) {
                            let _ = windows::hide_popover(app);
                        } else {
                            let _ = windows::show_popover(app);
                            let _ = tauri_plugin_positioner::WindowExt::move_window(
                                &win,
                                tauri_plugin_positioner::Position::TrayCenter,
                            );
                        }
                    }
                }
            }
        })
        .build(app)?;
    Ok(tray)
}
```

Register in `main.rs`'s `.setup(|app| ...)`:
```rust
tray::build(&app.handle())?;
```

Provide a placeholder `tray.png` (22×22 template image, white-on-transparent). If you don't have a real asset yet, use any small PNG — Task 23 finalizes assets.

Commit: `feat(app): tray icon with popover toggle and menu`.

---

### Task 18: Dock visibility + window lifecycle

**File:** `crates/stint-app/src/windows.rs`

Append:
```rust
#[cfg(target_os = "macos")]
pub fn hide_dock(app: &AppHandle) {
    use objc2_app_kit::NSApplicationActivationPolicy;
    unsafe {
        let cls = objc2::class!(NSApplication);
        let app_ns: *mut std::ffi::c_void =
            objc2::msg_send![cls, sharedApplication];
        let _: () = objc2::msg_send![
            app_ns as *mut objc2::runtime::AnyObject,
            setActivationPolicy: NSApplicationActivationPolicy::Accessory
        ];
    }
    let _ = app;
}

#[cfg(target_os = "macos")]
pub fn show_dock(app: &AppHandle) {
    use objc2_app_kit::NSApplicationActivationPolicy;
    unsafe {
        let cls = objc2::class!(NSApplication);
        let app_ns: *mut std::ffi::c_void =
            objc2::msg_send![cls, sharedApplication];
        let _: () = objc2::msg_send![
            app_ns as *mut objc2::runtime::AnyObject,
            setActivationPolicy: NSApplicationActivationPolicy::Regular
        ];
    }
    let _ = app;
}
```

Add `objc2` and `objc2-app-kit` to `crates/stint-app/Cargo.toml`:
```toml
[target.'cfg(target_os = "macos")'.dependencies]
objc2 = "0.5"
objc2-app-kit = { version = "0.2", features = ["NSApplication"] }
```

Wire dock visibility: in `main.rs`, after tray setup, call `hide_dock(&app.handle())` initially. When `show_main` is called, also call `show_dock`. When the main window closes, switch back to `hide_dock`.

Hook into the main window's `close_requested` event:
```rust
if let Some(main) = app.get_webview_window("main") {
    let app_handle = app.handle().clone();
    main.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = main_clone.hide();
            #[cfg(target_os = "macos")]
            windows::hide_dock(&app_handle);
        }
    });
}
```

(Hold `main_clone` from `main.clone()` outside the closure.)

Commit: `feat(app): dock visibility toggle on macOS`.

---

### Task 19: Cross-surface sync verification (manual)

Manual verification, no new files.

- [ ] **Step 1:** Build release binary `cargo build --release -p stint-app` and launch it. Confirm tray icon appears.
- [ ] **Step 2:** Click the tray icon. Confirm popover appears anchored to it.
- [ ] **Step 3:** In a terminal, run `stint start "from cli"`. Within ~1 second, the menu bar popover should show the running timer.
- [ ] **Step 4:** In the popover, click Stop. In the terminal, run `stint today`. The CLI should show the same entry with `end_at` set.
- [ ] **Step 5:** Open the main window from the tray menu. Verify entries list, settings panel work.

If any step fails, debug, fix, and document in the commit message. Otherwise:

Commit: `chore(app): verify cross-surface sync (manual smoke)` (empty commit if no code changes — use `git commit --allow-empty`).

---

### Task 20: README + run instructions

**File:** `README.md` (new)

```markdown
# stint

Time tracker with both a CLI (`stint`) and a macOS menu-bar app (`Stint.app`)
that sync with a self-hosted Solidtime instance.

## Phase 1 + 2 — current state

- CLI: `cargo install --path crates/stint-cli`
- GUI dev: `cd crates/stint-app && cargo tauri dev`
- GUI release: `cargo tauri build`

Both surfaces share `~/Library/Application Support/stint/stint.db`. Secrets live
in the macOS Keychain.

## Config

```
stint config set solidtime.url https://time.reyem.ca
stint config set solidtime.token   # prompts; goes to Keychain
stint config set solidtime.org <uuid>
stint config test                  # ping
```

Or use the Settings panel in the GUI.

## Sync model

Local-first. Mutations are persisted immediately and queued. A background worker
drains the queue against Solidtime with exponential backoff. If offline, work
queues up and flushes when the network returns.

## Status

- Phase 1: CLI + sync + crash recovery — shipped
- Phase 2: Tauri GUI + SolidJS UI — shipped
- Phase 3: Calendar integration (Google + MS + CalDAV) — not started
- Phase 4: Homebrew distribution — not started
```

Commit: `docs: README with current status`.

---

### Task 21: cargo fmt + clippy clean + pnpm typecheck

- [ ] **Step 1:** `cargo fmt --all` — let it reformat anything dirty.
- [ ] **Step 2:** `cargo clippy --workspace --all-targets -- -D warnings` — fix any warnings.
- [ ] **Step 3:** `cd ui && pnpm typecheck` — fix any TS errors.
- [ ] **Step 4:** Commit: `chore: fmt + clippy + typecheck clean across phase 2`.

---

### Task 22: Tag Phase 2 complete

```bash
git tag -a phase-2-complete -m "Phase 2 complete: Tauri GUI + SolidJS UI"
```

---

## Out of scope for Phase 2 (deferred)

These intentionally do not ship in Phase 2:

- **Project picker on the timer card.** v2 starts with description-only timers; project assignment via edit. Add to TimerCard in a polish task once Phase 3 is in flight.
- **Notification on long-running timer.** "You've been tracking for 1 hour" — needs macOS notification permissions and design.
- **Idle detection.** Detecting "user away from keyboard" requires accessibility permissions.
- **Global hotkey (e.g., ⌘⌥T to start/stop).** `tauri-plugin-global-shortcut` exists; defer to polish.
- **Theme toggle UI.** v1 follows system theme; explicit toggle deferred.
- **Background sync worker inside `stint-app`.** Phase 2 GUI runs `sync_now` on demand. A continuous background drain (Tokio task spawned at startup) is a Phase 3 prerequisite (calendar polling).
- **Phase 1 polish items** raised by Phase 1 reviewers — transaction atomicity in TimerService, backoff cap fix, etc. Tracked separately.

---

## Self-Review

**1. Spec coverage:**

| Spec section | Task |
|---|---|
| Cargo workspace `stint-app` | 1 |
| Tauri config (tray, two windows, identifier) | 2 |
| SolidJS + Tailwind 4 + Vite stack | 3 |
| Tauri↔SolidJS hello-world | 4 |
| All Tauri commands wired to stint-core | 5–10 |
| Tauri command smoke test | 11 |
| Typed UI API layer | 12 |
| Timer ticker (1s polling) | 13 |
| TimerCard, EntryList components | 14, 15 |
| Routes (popover/today/settings) | 16 |
| Tray icon + click-to-toggle | 17 |
| Dock visibility (macOS Accessory mode) | 18 |
| Cross-surface verification | 19 |
| Docs | 20 |
| Lint clean | 21 |
| Release tag | 22 |

**2. Placeholder scan:** None. Every Rust block and TypeScript block ships complete code.

**3. Type consistency check:** `EntryView`, `RunningTimerView`, `ConfigView`, `ProjectRow` on the Rust side match the corresponding TypeScript types in `ui/src/types.ts` (snake_case in Rust → snake_case in JSON → snake_case in TS, matching).

**4. Known mismatches to watch:**
- Tauri 2's testing utilities API has shifted across betas — Task 11 may need adaptation. If `tauri::test::get_ipc_response` is gone, fall back to a build-only test that just verifies `generate_handler!` compiles.
- `objc2-app-kit`'s API surface evolves; Task 18 may need a different call shape against the installed version. The fallback is to invoke AppleScript: `osascript -e 'tell application "System Events" to ...'` (worse, but works).

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-05-18-stint-phase-2-gui.md`. Same two options as Phase 1:

**1. Subagent-Driven** — fresh subagent per task, two-stage review between tasks
**2. Inline Execution** — execute in this session with batch checkpoints

Phase 1 ran subagent-driven and hit some API timeouts; Phase 2 has more frontend code where direct implementation may be faster. Suggest a hybrid: subagents for Rust-heavy tasks (1-11, 17-18), direct implementation for TypeScript-heavy tasks (3, 12-16) since the user can visually verify in the running dev server.

Which approach?
