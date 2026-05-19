import { HashRouter, Route } from "@solidjs/router";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useHotkey } from "./lib/useHotkey";
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

  // macOS-style shortcuts for the main window
  useHotkey("mod+,", () => navigate("/settings"));
  useHotkey("mod+1", () => navigate("/today"));
  useHotkey("mod+2", () => navigate("/settings"));
  useHotkey("mod+3", () => navigate("/about"));

  return (
    <HashRouter>
      <Route path="/today" component={Today} />
      <Route path="/settings" component={Settings} />
      <Route path="/about" component={About} />
      <Route path="*" component={Today} />
    </HashRouter>
  );
}
