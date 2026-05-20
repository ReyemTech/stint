import { createSignal, onCleanup, onMount, Show } from "solid-js";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { conflictResolve, type ConflictInfo } from "~/api";

export default function ConflictBanner() {
  const [conflict, setConflict] = createSignal<ConflictInfo | null>(null);
  const [busy, setBusy] = createSignal(false);
  let unlisten: UnlistenFn | undefined;

  onMount(async () => {
    unlisten = await listen<ConflictInfo>("pull:conflict", (e) => {
      setConflict(e.payload);
    });
  });

  onCleanup(() => unlisten?.());

  const handle = async (action: "stop_remote" | "switch" | "dismiss") => {
    const c = conflict();
    if (!c || busy()) return;
    setBusy(true);
    try {
      await conflictResolve(action, c.remote_id);
      setConflict(null);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Show when={conflict()}>
      {(c) => (
        <div class="mb-3 rounded-lg border border-amber-300 bg-amber-50 p-3 dark:border-amber-700 dark:bg-amber-950/40">
          <p class="text-sm text-amber-900 dark:text-amber-100">
            Another timer is running in Solidtime:{" "}
            <strong>"{c().remote_description}"</strong> started{" "}
            {new Date(c().remote_start_at).toLocaleTimeString()}.
          </p>
          <div class="mt-2 flex gap-2">
            <button
              class="rounded bg-amber-600 px-2 py-1 text-xs text-white hover:bg-amber-700 disabled:opacity-50"
              disabled={busy()}
              onClick={() => handle("stop_remote")}
            >
              Stop it remotely
            </button>
            <button
              class="rounded border border-amber-600 px-2 py-1 text-xs text-amber-700 hover:bg-amber-100 disabled:opacity-50 dark:border-amber-500 dark:text-amber-300 dark:hover:bg-amber-900/40"
              disabled={busy()}
              onClick={() => handle("switch")}
            >
              Switch to it
            </button>
            <button
              class="rounded px-2 py-1 text-xs text-amber-700 hover:bg-amber-100 disabled:opacity-50 dark:text-amber-300 dark:hover:bg-amber-900/40"
              disabled={busy()}
              onClick={() => handle("dismiss")}
            >
              Dismiss
            </button>
          </div>
        </div>
      )}
    </Show>
  );
}
