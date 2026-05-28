import { Show, createMemo, createResource, createSignal, onCleanup } from "solid-js";
import { useSearchParams } from "@solidjs/router";
import { listen } from "@tauri-apps/api/event";
import { api, pullNow } from "~/api";
import CalendarSection from "~/components/CalendarSection";
import ConflictBanner from "~/components/ConflictBanner";
import SyncErrorBanner from "~/components/SyncErrorBanner";
import Duration from "~/components/Duration";
import EntryList from "~/components/EntryList";
import IdleBanner from "~/components/IdleBanner";
import MainNav from "~/components/MainNav";
import TimerCard from "~/components/TimerCard";
import SectionLabel from "~/components/ui/SectionLabel";
import StatusDot from "~/components/ui/StatusDot";
import { sumCompletedEntrySeconds } from "~/lib/entryFormat";
import { useTimerStore } from "~/stores/timer";

export default function Today() {
  const timer = useTimerStore();
  const [searchParams] = useSearchParams();
  // `?entry=<uuid>` set by the stint:// deep-link handler (Spotlight taps
  // and similar) — EntryList uses it to scroll the matching row into
  // view + briefly highlight it.
  const focusUuid = () => {
    const raw = searchParams.entry;
    return typeof raw === "string" ? raw : undefined;
  };
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

  const totalSeconds = createMemo(
    () =>
      (timer.running() ? timer.elapsedSecs() : 0) +
      sumCompletedEntrySeconds(entries() ?? []),
  );

  const billableSeconds = createMemo(
    () =>
      (timer.running() && timer.running()!.billable ? timer.elapsedSecs() : 0) +
      sumCompletedEntrySeconds(entries() ?? [], { onlyBillable: true }),
  );

  async function syncNow() {
    setSyncing(true);
    setSyncMsg(null);
    try {
      // Pull first so the UI reflects remote state even if push fails
      // (e.g. validation error on a queued op). Then drain the push queue.
      await pullNow();
      const n = await api.syncNow();
      setSyncMsg(n > 0 ? `Synced ${n} item${n === 1 ? "" : "s"}` : "Synced");
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
          <MainNav
            active="today"
            leading={
              <SyncBadge
                syncing={syncing()}
                pending={pending()}
                onClick={syncNow}
              />
            }
          />
        </header>

        <ConflictBanner />
        <SyncErrorBanner />

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
            accent
          />
        </div>

        <IdleBanner onChange={() => refetch()} />

        <TimerCard />

        <CalendarSection onEntriesChanged={() => refetch()} />

        <section class="mt-8">
          <div class="mb-3 flex items-baseline justify-between">
            <SectionLabel>Entries</SectionLabel>
            <span class="text-xs text-zinc-400 dark:text-zinc-500">
              {(entries() ?? []).length} total
            </span>
          </div>
          <Show
            when={!entries.loading}
            fallback={
              <p class="rounded-2xl border border-dashed border-zinc-200 py-8 text-center text-sm text-zinc-400 dark:border-zinc-800">
                Loading…
              </p>
            }
          >
            <div class="rounded-2xl border border-black/[0.06] bg-white dark:border-white/[0.06] dark:bg-zinc-900">
              <EntryList
                entries={entries() ?? []}
                focusUuid={focusUuid()}
                onChange={() => refetch()}
              />
            </div>
          </Show>
        </section>
      </div>
    </div>
  );
}

function Stat(props: { label: string; value: any; accent?: boolean }) {
  return (
    <div class="rounded-2xl border border-black/[0.06] bg-white px-4 py-3 dark:border-white/[0.06] dark:bg-zinc-900">
      <SectionLabel>{props.label}</SectionLabel>
      <div
        class="mt-1 text-2xl font-semibold tracking-tight tabular-nums"
        classList={{
          "text-emerald-600 dark:text-emerald-400": props.accent,
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
  const tone = () => {
    if (props.syncing || props.pending > 0) return "amber" as const;
    return "emerald" as const;
  };
  return (
    <button
      class="mr-2 inline-flex items-center gap-1.5 rounded-md px-2.5 py-1.5 text-zinc-600 transition hover:bg-zinc-100 hover:text-zinc-900 disabled:opacity-50 dark:text-zinc-300 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
      classList={{
        "text-amber-700 dark:text-amber-400": props.pending > 0 && !props.syncing,
      }}
      disabled={props.syncing}
      onClick={props.onClick}
      title="Sync with Solidtime (push pending, pull remote changes)"
    >
      <StatusDot tone={tone()} ping={props.syncing} />
      {props.syncing
        ? "Syncing…"
        : props.pending > 0
          ? `Sync (${props.pending})`
          : "Synced"}
    </button>
  );
}
