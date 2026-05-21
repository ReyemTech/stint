import { For, Show, createMemo, createResource, onCleanup } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { api, calendarApi } from "~/api";
import Accordion from "~/components/ui/Accordion";
import Button from "~/components/ui/Button";
import Pill from "~/components/ui/Pill";
import { formatEventTime } from "~/lib/entryFormat";
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

  // Map calendar_id → default project name. Lets each event show which
  // project it'll be logged to. Built lazily from all accounts' calendars
  // + the project list; empty map until both resolve.
  const [calendarsByAccount] = createResource(
    () => (accounts() ?? []).map((a) => a.id).join(","),
    async (): Promise<Map<string, string>> => {
      const list = accounts() ?? [];
      const out = new Map<string, string>();
      for (const a of list) {
        try {
          const cals = await calendarApi.listCalendars(a.id);
          for (const c of cals) {
            if (c.default_project_id) {
              out.set(c.id, c.default_project_id);
            }
          }
        } catch {
          // ignore per-account failure
        }
      }
      return out;
    },
  );

  const [projects] = createResource(() => api.listProjects(), {
    initialValue: [],
  });

  const projectNameById = createMemo(() => {
    const m = new Map<string, string>();
    for (const p of projects() ?? []) m.set(p.id, p.name);
    return m;
  });

  const defaultProjectForCalendar = (calendarId: string): string | null => {
    const pid = calendarsByAccount()?.get(calendarId);
    if (!pid) return null;
    return projectNameById().get(pid) ?? null;
  };

  const unlistenChanged = listen("calendar:changed", () => refetch());
  onCleanup(() => {
    unlistenChanged.then((fn) => fn()).catch(() => {});
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

  async function handleRevert(g: EventByAccount, e: CalendarEventWithDecision) {
    try {
      await calendarApi.revertEvent(g.account.id, e.id, e.start_at);
      props.onEntriesChanged();
      refetch();
    } catch (err) {
      console.error("Revert failed:", err);
    }
  }

  return (
    <Show when={total() > 0}>
      <div class="mt-8">
        <Accordion
          title="Calendar"
          right={
            <span class="text-xs text-zinc-400 dark:text-zinc-500">
              {total()} event{total() === 1 ? "" : "s"} today
            </span>
          }
        >
          <div class="space-y-2">
            <For each={groups() ?? []}>
              {(g) => (
                <For each={g.events}>
                  {(e) => (
                    <EventRow
                      event={e}
                      defaultProjectName={defaultProjectForCalendar(e.calendar_id)}
                      onLog={() => handleLog(g, e)}
                      onIgnore={() => handleIgnore(g, e)}
                      onRevert={() => handleRevert(g, e)}
                    />
                  )}
                </For>
              )}
            </For>
          </div>
        </Accordion>
      </div>
    </Show>
  );
}

function EventRow(props: {
  event: CalendarEventWithDecision;
  defaultProjectName: string | null;
  onLog: () => void;
  onIgnore: () => void;
  onRevert: () => void;
}) {
  const decided = () => props.event.decision !== null;
  const logged = () =>
    props.event.decision === "logged_manual" ||
    props.event.decision === "logged_auto";

  const startLabel = () =>
    formatEventTime(props.event.start_at, props.event.is_all_day);
  const endLabel = () =>
    formatEventTime(props.event.end_at, props.event.is_all_day);

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
        <Show when={!decided() && props.defaultProjectName}>
          <Pill tone="indigo">→ {props.defaultProjectName}</Pill>
        </Show>
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
      <Show when={decided()}>
        <Button
          variant="ghost"
          size="sm"
          onClick={props.onRevert}
          title={
            logged()
              ? "Undo: delete the logged entry and restore Log/Ignore"
              : "Undo: restore Log this / Ignore actions"
          }
        >
          <svg
            class="h-4 w-4"
            viewBox="0 0 20 20"
            fill="none"
            stroke="currentColor"
            stroke-width="1.75"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
          >
            <path d="M8 13L3 8l5-5" />
            <path d="M3 8h8a5 5 0 0 1 0 10h-1" />
          </svg>
          <span class="sr-only">Undo</span>
        </Button>
      </Show>
    </div>
  );
}

