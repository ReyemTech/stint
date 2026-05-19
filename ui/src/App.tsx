import { HashRouter, Route } from "@solidjs/router";
import { getCurrentWindow } from "@tauri-apps/api/window";
import Popover from "./routes/Popover";
import Settings from "./routes/Settings";
import Today from "./routes/Today";

const isPopover = getCurrentWindow().label === "popover";

if (isPopover) {
  document.body.classList.add("popover-window");
}

export default function App() {
  if (isPopover) {
    return <Popover />;
  }
  return (
    <HashRouter>
      <Route path="/today" component={Today} />
      <Route path="/settings" component={Settings} />
      <Route path="*" component={Today} />
    </HashRouter>
  );
}
