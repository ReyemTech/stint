import { formatDuration } from "./Duration";
import type { Entry } from "~/types";

function durationSecs(start: string, end: string | null): number {
  const s = new Date(start).getTime();
  const e = end ? new Date(end).getTime() : Date.now();
  return Math.max(0, Math.floor((e - s) / 1000));
}

export default function EntryRow(props: {
  entry: Entry;
  onDelete?: (id: string) => void;
}) {
  const isRunning = !props.entry.end_at;
  return (
    <li class="flex items-center justify-between border-b border-zinc-100 py-2 dark:border-zinc-800">
      <div class="min-w-0">
        <div class="truncate text-sm">{props.entry.description}</div>
        <div class="text-xs text-zinc-500">
          <span
            class="rounded px-1 text-[10px] font-medium uppercase"
            classList={{
              "bg-zinc-100 text-zinc-600 dark:bg-zinc-800": !isRunning,
              "bg-green-100 text-green-700 dark:bg-green-950 dark:text-green-300":
                isRunning,
            }}
          >
            {isRunning ? "Running" : props.entry.sync_state}
          </span>
        </div>
      </div>
      <div class="flex items-center gap-3">
        <span class="font-mono tabular-nums text-sm">
          {formatDuration(durationSecs(props.entry.start_at, props.entry.end_at))}
        </span>
        <button
          class="text-xs text-zinc-400 hover:text-red-600"
          onClick={() => props.onDelete?.(props.entry.local_uuid)}
        >
          Delete
        </button>
      </div>
    </li>
  );
}
