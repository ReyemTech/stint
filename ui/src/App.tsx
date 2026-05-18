import { Route, Router } from "@solidjs/router";
import Popover from "./routes/Popover";
import Settings from "./routes/Settings";
import Today from "./routes/Today";

export default function App() {
  return (
    <Router>
      <Route path="/popover" component={Popover} />
      <Route path="/today" component={Today} />
      <Route path="/settings" component={Settings} />
    </Router>
  );
}
