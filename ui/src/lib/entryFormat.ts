import type { Entry } from "~/types";
import type { PillTone } from "~/components/ui/Pill";
import type { DotTone } from "~/components/ui/StatusDot";

/// Duration of an entry in seconds. If `end_at` is null (still running),
/// uses now as the end. Floored to whole seconds, never negative.
export function entryDurationSecs(start: string, end: string | null): number {
  const s = new Date(start).getTime();
  const e = end ? new Date(end).getTime() : Date.now();
  return Math.max(0, Math.floor((e - s) / 1000));
}

/// Visual metadata for an entry row given its sync_state and whether it
/// is currently running. Used by EntryRow to render the pill + status dot.
export function entrySyncMeta(
  state: Entry["sync_state"],
  isRunning: boolean,
): { text: string; tone: PillTone; dotTone: DotTone } {
  if (isRunning) return { text: "Running", tone: "emerald", dotTone: "emerald" };
  switch (state) {
    case "synced":
      return { text: "Synced", tone: "emerald", dotTone: "emerald" };
    case "dirty":
      return { text: "Edited", tone: "amber", dotTone: "amber" };
    case "pending_create":
      return { text: "Pending", tone: "amber", dotTone: "amber" };
    case "pending_delete":
      return { text: "Deleting", tone: "red", dotTone: "red" };
  }
}

/// Sum durations across completed entries. Optionally filter to billable
/// ones only. Used by Today + Popover for the day-total displays.
export function sumCompletedEntrySeconds(
  entries: Entry[],
  opts: { onlyBillable?: boolean } = {},
): number {
  let total = 0;
  for (const e of entries) {
    if (!e.end_at) continue;
    if (opts.onlyBillable && !e.billable) continue;
    total += entryDurationSecs(e.start_at, e.end_at);
  }
  return total;
}

/// Format an ISO timestamp as a local HH:MM string (e.g. "09:30"). Returns
/// an empty string for all-day events. Used by CalendarSection to render
/// the time-of-day next to each event title.
export function formatEventTime(iso: string, allDay: boolean): string {
  if (allDay) return "";
  const d = new Date(iso);
  return d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}
