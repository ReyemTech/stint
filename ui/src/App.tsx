import { HashRouter, Route } from "@solidjs/router";
import Popover from "./routes/Popover";
import Settings from "./routes/Settings";
import Today from "./routes/Today";

export default function App() {
  return (
    <HashRouter>
      <Route path="/popover" component={Popover} />
      <Route path="/today" component={Today} />
      <Route path="/settings" component={Settings} />
      <Route path="*" component={Today} />
    </HashRouter>
  );
}
