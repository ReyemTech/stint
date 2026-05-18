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
