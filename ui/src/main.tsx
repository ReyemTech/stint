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
