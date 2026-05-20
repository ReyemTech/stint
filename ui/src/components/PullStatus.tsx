import { createSignal, onMount, Show } from "solid-js";
import { pullNow } from "~/api";

export default function PullStatus() {
  const [lastPulledAt, setLastPulledAt] = createSignal<Date | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const refresh = async () => {
    if (busy()) return;
    setBusy(true);
    setError(null);
    try {
      await pullNow();
      setLastPulledAt(new Date());
    } catch (e) {
      const msg = e && typeof e === "object" && "message" in e
        ? String((e as { message: unknown }).message)
        : String(e);
      setError(msg);
    } finally {
      setBusy(false);
    }
  };

  onMount(refresh);

  const ago = () => {
    const t = lastPulledAt();
    if (!t) return "—";
    const secs = Math.floor((Date.now() - t.getTime()) / 1000);
    if (secs < 60) return `${secs}s ago`;
    return `${Math.floor(secs / 60)}m ago`;
  };

  return (
    <div class="mb-3 flex items-center gap-2 text-xs text-zinc-500 dark:text-zinc-400">
      <span>Last pulled: {ago()}</span>
      <button
        class="rounded border border-zinc-200 px-1.5 py-0.5 hover:bg-zinc-100 disabled:opacity-50 dark:border-zinc-700 dark:hover:bg-zinc-800"
        disabled={busy()}
        onClick={refresh}
      >
        {busy() ? "Refreshing…" : "Refresh"}
      </button>
      <Show when={error()}>
        <span class="text-rose-500">{error()}</span>
      </Show>
    </div>
  );
}
