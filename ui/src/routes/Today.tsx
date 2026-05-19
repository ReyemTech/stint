import { Show, createMemo, createResource, createSignal, onCleanup } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { api } from "~/api";
import EntryList from "~/components/EntryList";
import TimerCard from "~/components/TimerCard";
import Duration from "~/components/Duration";
import { useTimerStore } from "~/stores/timer";
import { openSolidtime } from "~/lib/openSolidtime";

export default function Today() {
  const timer = useTimerStore();
  const [entries, { refetch }] = createResource(() => api.listToday());
  const [syncing, setSyncing] = createSignal(false);
  const [syncMsg, setSyncMsg] = createSignal<string | null>(null);

  const unlisten = listen("entries:changed", () => refetch());
  onCleanup(() => {
    unlisten.then((fn) => fn()).catch(() => {});
  });

  const pending = createMemo(() =>
    (entries() ?? []).filter((e) => e.sync_state !== "synced").length,
  );

  const totalSeconds = createMemo(() => {
    let total = timer.running() ? timer.elapsedSecs() : 0;
    for (const e of entries() ?? []) {
      if (!e.end_at) continue;
      const s = new Date(e.start_at).getTime();
      const f = new Date(e.end_at).getTime();
      total += Math.max(0, Math.floor((f - s) / 1000));
    }
    return total;
  });

  const billableSeconds = createMemo(() => {
    let total =
      timer.running() && timer.running()!.billable ? timer.elapsedSecs() : 0;
    for (const e of entries() ?? []) {
      if (!e.end_at || !e.billable) continue;
      const s = new Date(e.start_at).getTime();
      const f = new Date(e.end_at).getTime();
      total += Math.max(0, Math.floor((f - s) / 1000));
    }
    return total;
  });

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
      setTimeout(() => setSyncMsg(null), 4000);
    }
  }

  return (
    <div class="min-h-screen bg-zinc-50/60 dark:bg-zinc-950">
      <div class="mx-auto max-w-3xl px-6 py-8">
        <header class="mb-6 flex items-center justify-between">
          <div>
            <h1 class="text-2xl font-semibold tracking-tight">Today</h1>
            <p class="mt-0.5 text-xs text-zinc-500 dark:text-zinc-400">
              {new Date().toLocaleDateString(undefined, {
                weekday: "long",
                month: "long",
                day: "numeric",
              })}
            </p>
          </div>
          <nav class="flex items-center gap-1 text-xs">
            <SyncBadge
              syncing={syncing()}
              pending={pending()}
              onClick={syncNow}
            />
            <a
              class="rounded-md px-2.5 py-1.5 text-zinc-500 transition hover:bg-zinc-100 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
              href="#/today"
            >
              Today
            </a>
            <a
              class="rounded-md px-2.5 py-1.5 text-zinc-500 transition hover:bg-zinc-100 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
              href="#/settings"
            >
              Settings
            </a>
            <button
              class="rounded-md px-2.5 py-1.5 text-zinc-500 transition hover:bg-zinc-100 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
              onClick={() => openSolidtime()}
              title="Open Solidtime in browser"
            >
              Solidtime ↗
            </button>
          </nav>
        </header>

        <Show when={syncMsg()}>
          <div class="mb-3 text-xs text-zinc-500 dark:text-zinc-400">
            {syncMsg()}
          </div>
        </Show>

        <div class="mb-6 grid grid-cols-2 gap-3">
          <Stat label="Total today" value={<Duration seconds={totalSeconds()} />} />
          <Stat
            label="Billable"
            value={<Duration seconds={billableSeconds()} />}
            accent="emerald"
          />
        </div>

        <TimerCard />

        <section class="mt-8">
          <div class="mb-3 flex items-baseline justify-between">
            <h2 class="text-sm font-semibold uppercase tracking-[0.08em] text-zinc-400 dark:text-zinc-500">
              Entries
            </h2>
            <span class="text-xs text-zinc-400 dark:text-zinc-500">
              {(entries() ?? []).length} total
            </span>
          </div>
          <Show
            when={!entries.loading}
            fallback={
              <p class="rounded-xl border border-dashed border-zinc-200 py-8 text-center text-sm text-zinc-400 dark:border-zinc-800">
                Loading…
              </p>
            }
          >
            <div class="rounded-2xl border border-black/[0.06] bg-white dark:border-white/[0.06] dark:bg-zinc-900">
              <EntryList
                entries={entries() ?? []}
                onChange={() => refetch()}
                onDelete={async (id) => {
                  await api.deleteEntry(id);
                  refetch();
                }}
              />
            </div>
          </Show>
        </section>
      </div>
    </div>
  );
}

function Stat(props: {
  label: string;
  value: any;
  accent?: "emerald" | "indigo";
}) {
  return (
    <div class="rounded-2xl border border-black/[0.06] bg-white px-4 py-3 dark:border-white/[0.06] dark:bg-zinc-900">
      <div class="text-[10px] font-semibold uppercase tracking-[0.08em] text-zinc-400 dark:text-zinc-500">
        {props.label}
      </div>
      <div
        class="mt-1 text-2xl font-semibold tracking-tight tabular-nums"
        classList={{
          "text-emerald-600 dark:text-emerald-400": props.accent === "emerald",
        }}
      >
        {props.value}
      </div>
    </div>
  );
}

function SyncBadge(props: {
  syncing: boolean;
  pending: number;
  onClick: () => void;
}) {
  return (
    <button
      class="mr-2 inline-flex items-center gap-1.5 rounded-md px-2.5 py-1.5 text-zinc-600 transition hover:bg-zinc-100 hover:text-zinc-900 disabled:opacity-50 dark:text-zinc-300 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
      classList={{
        "text-amber-700 dark:text-amber-400": props.pending > 0 && !props.syncing,
      }}
      disabled={props.syncing}
      onClick={props.onClick}
      title="Push pending entries to Solidtime"
    >
      <span
        class="h-1.5 w-1.5 rounded-full"
        classList={{
          "bg-amber-500 animate-pulse": props.syncing,
          "bg-amber-500": props.pending > 0 && !props.syncing,
          "bg-emerald-500": props.pending === 0 && !props.syncing,
        }}
      />
      {props.syncing
        ? "Syncing…"
        : props.pending > 0
          ? `Sync (${props.pending})`
          : "Synced"}
    </button>
  );
}
