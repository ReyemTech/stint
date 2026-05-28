import { Show, createSignal, onCleanup, onMount } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { api } from "~/api";

interface IdleEvent {
  idle_started: string;
  idle_secs: number;
}

export default function IdleBanner(props: { onChange?: () => void }) {
  const [event, setEvent] = createSignal<IdleEvent | null>(null);
  const [busy, setBusy] = createSignal(false);
  let dismissTimer: number | undefined;

  onMount(async () => {
    const unlisten = await listen<IdleEvent>("idle:detected", (e) => {
      setEvent(e.payload);
      if (dismissTimer) window.clearTimeout(dismissTimer);
      dismissTimer = window.setTimeout(() => setEvent(null), 5 * 60 * 1000);
    });
    onCleanup(() => {
      unlisten();
      if (dismissTimer) window.clearTimeout(dismissTimer);
    });
  });

  function fmtMinutes(secs: number): string {
    const m = Math.round(secs / 60);
    return `${m} minute${m === 1 ? "" : "s"}`;
  }

  async function handleKeep() {
    setBusy(true);
    try {
      await api.idleKeep();
    } finally {
      setBusy(false);
      setEvent(null);
    }
  }

  async function handleDiscard() {
    const e = event();
    if (!e) return;
    setBusy(true);
    try {
      await api.idleDiscard(e.idle_started);
      props.onChange?.();
    } finally {
      setBusy(false);
      setEvent(null);
    }
  }

  async function handleSplit() {
    const e = event();
    if (!e) return;
    setBusy(true);
    try {
      await api.idleSplit(e.idle_started);
      props.onChange?.();
    } finally {
      setBusy(false);
      setEvent(null);
    }
  }

  return (
    <Show when={event()}>
      {(e) => (
        <div class="mb-3 rounded-2xl border border-amber-300 bg-amber-50 px-4 py-3 dark:border-amber-700 dark:bg-amber-950/40">
          <div class="text-sm font-medium text-amber-900 dark:text-amber-100">
            ⏸ You were idle for {fmtMinutes(e().idle_secs)}
          </div>
          <div class="mt-2 flex gap-2">
            <button
              type="button"
              class="rounded-md bg-zinc-200 px-3 py-1 text-xs font-medium hover:bg-zinc-300 dark:bg-zinc-700 dark:hover:bg-zinc-600 disabled:opacity-50"
              disabled={busy()}
              onClick={handleKeep}
            >
              Keep
            </button>
            <button
              type="button"
              class="rounded-md bg-amber-600 px-3 py-1 text-xs font-medium text-white hover:bg-amber-700 disabled:opacity-50"
              disabled={busy()}
              onClick={handleDiscard}
            >
              Discard {fmtMinutes(e().idle_secs)}
            </button>
            <button
              type="button"
              class="rounded-md bg-emerald-600 px-3 py-1 text-xs font-medium text-white hover:bg-emerald-700 disabled:opacity-50"
              disabled={busy()}
              onClick={handleSplit}
            >
              Discard + restart now
            </button>
          </div>
        </div>
      )}
    </Show>
  );
}
