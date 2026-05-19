import { Show, createResource, createSignal } from "solid-js";
import { api } from "~/api";
import EntryList from "~/components/EntryList";
import TimerCard from "~/components/TimerCard";

export default function Today() {
  const [entries, { refetch }] = createResource(() => api.listToday());
  const [syncing, setSyncing] = createSignal(false);
  const [syncMsg, setSyncMsg] = createSignal<string | null>(null);

  // Count unsynced entries for the badge.
  const pending = () =>
    (entries() ?? []).filter((e) => e.sync_state !== "synced").length;

  async function syncNow() {
    setSyncing(true);
    setSyncMsg(null);
    try {
      const n = await api.syncNow();
      setSyncMsg(n > 0 ? `Synced ${n} item${n === 1 ? "" : "s"}` : "All synced");
      refetch();
    } catch (e) {
      setSyncMsg(`Sync failed: ${(e as { message: string }).message}`);
    } finally {
      setSyncing(false);
    }
  }

  return (
    <div class="mx-auto max-w-2xl p-6">
      <header class="mb-4 flex items-baseline justify-between">
        <h1 class="text-lg font-semibold">Today</h1>
        <div class="flex items-center gap-4">
          <button
            class="flex items-center gap-1.5 rounded-md border border-zinc-300 px-2 py-1 text-xs text-zinc-700 transition hover:bg-zinc-50 disabled:opacity-50 dark:border-zinc-700 dark:text-zinc-300 dark:hover:bg-zinc-800"
            onClick={syncNow}
            disabled={syncing()}
            title="Push pending entries to Solidtime"
          >
            <span
              class="inline-block h-1.5 w-1.5 rounded-full"
              classList={{
                "bg-amber-500": pending() > 0,
                "bg-green-500": pending() === 0,
                "animate-pulse": syncing(),
              }}
            />
            {syncing() ? "Syncing…" : pending() > 0 ? `Sync (${pending()})` : "Synced"}
          </button>
          <nav class="text-xs text-zinc-500">
            <a class="mr-3 hover:underline" href="#/today">
              Today
            </a>
            <a class="hover:underline" href="#/settings">
              Settings
            </a>
          </nav>
        </div>
      </header>

      <Show when={syncMsg()}>
        <div class="mb-3 text-xs text-zinc-500">{syncMsg()}</div>
      </Show>

      <TimerCard />

      <section class="mt-6">
        <h2 class="mb-2 text-sm font-medium text-zinc-700 dark:text-zinc-300">
          Entries
        </h2>
        <Show
          when={!entries.loading}
          fallback={<p class="text-sm text-zinc-500">Loading…</p>}
        >
          <EntryList
            entries={entries() ?? []}
            onChange={() => refetch()}
            onDelete={async (id) => {
              await api.deleteEntry(id);
              refetch();
            }}
          />
        </Show>
      </section>
    </div>
  );
}
