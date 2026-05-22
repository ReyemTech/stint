import { HashRouter, Route } from "@solidjs/router";
import { Show } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useHotkey } from "./lib/useHotkey";
import { useUpdateBanner } from "./lib/updateBanner";
import About from "./routes/About";
import Popover from "./routes/Popover";
import Settings from "./routes/Settings";
import Today from "./routes/Today";

const isPopover = getCurrentWindow().label === "popover";

if (isPopover) {
  document.body.classList.add("popover-window");
} else {
  // Main window: listen for navigation events from Rust (e.g. tray menu
  // selecting "About Stint" → reopen + nav).
  listen<string>("navigate", (e) => {
    if (typeof e.payload === "string") {
      window.location.hash = e.payload;
    }
  }).catch(() => {});
}

function navigate(path: string) {
  window.location.hash = path;
}

export default function App() {
  if (isPopover) {
    useHotkey("esc", async () => {
      try {
        await getCurrentWindow().hide();
      } catch {
        /* ignore */
      }
    });
    return <Popover />;
  }

  // ⌘, is owned by the native macOS menu (Settings…). Number shortcuts
  // are convenience switches between the three main routes.
  useHotkey("mod+1", () => navigate("/today"));
  useHotkey("mod+2", () => navigate("/settings"));
  useHotkey("mod+3", () => navigate("/about"));

  // Auto-poll for updates (5s after mount, then every 24h). The banner is
  // mounted above the router so it persists across route changes and never
  // gets unmounted by navigation — important because we don't want to
  // restart the polling timer every time the user clicks a tab.
  const updateInfo = useUpdateBanner();

  return (
    <>
      <Show when={updateInfo()?.available}>
        <div class="flex items-center justify-between gap-3 bg-sky-600 px-4 py-1.5 text-xs text-white dark:bg-sky-700">
          <span>
            stint v{updateInfo()!.latest_version} is available.
          </span>
          <button
            type="button"
            class="rounded bg-sky-700 px-2 py-0.5 font-medium hover:bg-sky-800 dark:bg-sky-800 dark:hover:bg-sky-900"
            onClick={() => navigate("/settings")}
          >
            View update
          </button>
        </div>
      </Show>
      <HashRouter>
        <Route path="/today" component={Today} />
        <Route path="/settings" component={Settings} />
        <Route path="/about" component={About} />
        <Route path="*" component={Today} />
      </HashRouter>
    </>
  );
}
