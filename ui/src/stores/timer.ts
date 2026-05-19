import { createSignal, onCleanup } from "solid-js";
import { listen } from "@tauri-apps/api/event";
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

  // 1s tick keeps the elapsed counter live. Event listener makes
  // start/stop from another window propagate instantly instead of waiting
  // up to a full second.
  const id = window.setInterval(refresh, 1000);
  const unlisten = listen("entries:changed", () => refresh());
  refresh();
  onCleanup(() => {
    window.clearInterval(id);
    unlisten.then((fn) => fn()).catch(() => {});
  });

  return {
    running,
    elapsedSecs,
    refresh,
    async start(description: string, projectId?: string, billable = false) {
      await api.startTimer(description, projectId ?? null, null, billable);
      await refresh();
    },
    async stop() {
      await api.stopTimer();
      await refresh();
    },
  };
}
