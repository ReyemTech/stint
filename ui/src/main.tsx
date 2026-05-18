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
