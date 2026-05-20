import { For, Show, createMemo, createResource, onCleanup } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { calendarApi } from "~/api";
import Button from "~/components/ui/Button";
import Pill from "~/components/ui/Pill";
import SectionLabel from "~/components/ui/SectionLabel";
import type {
  CalendarAccount,
  CalendarEventWithDecision,
} from "~/types";

type EventByAccount = {
  account: CalendarAccount;
  events: CalendarEventWithDecision[];
};

export default function CalendarSection(props: { onEntriesChanged: () => void }) {
  const [accounts] = createResource(() => calendarApi.listAccounts());

  const todayRange = createMemo(() => {
    const now = new Date();
    const start = new Date(now);
    start.setHours(0, 0, 0, 0);
    const end = new Date(start);
    end.setDate(end.getDate() + 1);
    return { from: start.toISOString(), to: end.toISOString() };
  });

  const [groups, { refetch }] = createResource(
    () => (accounts() ?? []).map((a) => a.id).join(","),
    async (): Promise<EventByAccount[]> => {
      const list = accounts() ?? [];
      const range = todayRange();
      const result: EventByAccount[] = [];
      for (const account of list) {
        try {
          const events = await calendarApi.listEventsInRange(
            account.id,
            range.from,
            range.to,
          );
          result.push({ account, events });
        } catch {
          result.push({ account, events: [] });
        }
      }
      return result;
    },
  );

  const unlisten = listen("calendar:changed", () => refetch());
  onCleanup(() => {
    unlisten.then((fn) => fn()).catch(() => {});
  });

  const total = createMemo(() =>
    (groups() ?? []).reduce((acc, g) => acc + g.events.length, 0),
  );

  async function handleLog(g: EventByAccount, e: CalendarEventWithDecision) {
    try {
      await calendarApi.logEvent(g.account.id, e.id, e.start_at);
      props.onEntriesChanged();
      refetch();
    } catch (err) {
      console.error("Log this failed:", err);
    }
  }

  async function handleIgnore(g: EventByAccount, e: CalendarEventWithDecision) {
    try {
      await calendarApi.ignoreEvent(g.account.id, e.id, e.start_at);
      refetch();
    } catch (err) {
      console.error("Ignore failed:", err);
    }
  }

  return (
    <Show when={total() > 0}>
      <section class="mt-8">
        <div class="mb-3 flex items-baseline justify-between">
          <SectionLabel>Calendar</SectionLabel>
          <span class="text-xs text-zinc-400 dark:text-zinc-500">
            {total()} event{total() === 1 ? "" : "s"} today
          </span>
        </div>

        <div class="space-y-2">
          <For each={groups() ?? []}>
            {(g) => (
              <For each={g.events}>
                {(e) => (
                  <EventRow
                    event={e}
                    onLog={() => handleLog(g, e)}
                    onIgnore={() => handleIgnore(g, e)}
                  />
                )}
              </For>
            )}
          </For>
        </div>
      </section>
    </Show>
  );
}

function EventRow(props: {
  event: CalendarEventWithDecision;
  onLog: () => void;
  onIgnore: () => void;
}) {
  const decided = () => props.event.decision !== null;
  const logged = () =>
    props.event.decision === "logged_manual" ||
    props.event.decision === "logged_auto";

  const startLabel = () => formatTime(props.event.start_at, props.event.is_all_day);
  const endLabel = () => formatTime(props.event.end_at, props.event.is_all_day);

  return (
    <div
      class="flex items-center justify-between rounded-lg border border-black/[0.06] bg-white px-3 py-2 dark:border-white/[0.06] dark:bg-zinc-900"
      classList={{ "opacity-50": decided() && !logged() }}
    >
      <div class="flex min-w-0 flex-1 items-center gap-3">
        <div class="w-24 shrink-0 text-xs tabular-nums text-zinc-500">
          {props.event.is_all_day ? "all-day" : `${startLabel()} – ${endLabel()}`}
        </div>
        <div class="min-w-0 flex-1 truncate text-sm">{props.event.title}</div>
        <Show when={logged()}>
          <Pill tone="emerald">Logged</Pill>
        </Show>
        <Show when={props.event.decision === "ignored"}>
          <Pill tone="neutral">Ignored</Pill>
        </Show>
      </div>

      <Show when={!decided()}>
        <div class="ml-2 flex items-center gap-1">
          <Button variant="ghost" size="sm" onClick={props.onLog}>
            Log this
          </Button>
          <Button variant="ghost" size="sm" onClick={props.onIgnore}>
            Ignore
          </Button>
        </div>
      </Show>
    </div>
  );
}

function formatTime(iso: string, allDay: boolean): string {
  if (allDay) return "";
  const d = new Date(iso);
  return d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}
