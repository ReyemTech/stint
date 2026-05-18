import { Show, createResource } from "solid-js";
import { api } from "~/api";
import EntryList from "~/components/EntryList";
import TimerCard from "~/components/TimerCard";

export default function Today() {
  const [entries, { refetch }] = createResource(() => api.listToday());

  return (
    <div class="mx-auto max-w-2xl p-6">
      <header class="mb-4 flex items-baseline justify-between">
        <h1 class="text-lg font-semibold">Today</h1>
        <nav class="text-xs text-zinc-500">
          <a class="mr-3 hover:underline" href="/#/today">Today</a>
          <a class="hover:underline" href="/#/settings">Settings</a>
        </nav>
      </header>

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
