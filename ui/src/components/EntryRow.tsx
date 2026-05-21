import { Show, createSignal } from "solid-js";
import { api } from "~/api";
import { entryDurationSecs, entrySyncMeta } from "~/lib/entryFormat";
import type { Entry } from "~/types";
import { formatDuration } from "./Duration";
import EditEntryDialog from "./EditEntryDialog";
import Pill from "./ui/Pill";
import StatusDot from "./ui/StatusDot";

export default function EntryRow(props: {
  entry: Entry;
  projectName?: string;
  isFirst?: boolean;
  /// Fires after any save or delete in the dialog. Callers refetch here.
  onChange?: () => void;
}) {
  const [editing, setEditing] = createSignal(false);
  const [restarting, setRestarting] = createSignal(false);
  const isRunning = !props.entry.end_at;
  const meta = () => entrySyncMeta(props.entry.sync_state, isRunning);

  async function handleRestart() {
    if (restarting()) return;
    setRestarting(true);
    try {
      await api.restartEntry(props.entry.local_uuid);
      props.onChange?.();
    } catch (e) {
      console.error("restart_entry failed:", e);
    } finally {
      setRestarting(false);
    }
  }

  return (
    <li
      class="flex items-center transition hover:bg-zinc-50 dark:hover:bg-zinc-800/40"
      classList={{
        "border-t border-black/[0.04] dark:border-white/[0.04]": !props.isFirst,
      }}
    >
      <button
        type="button"
        class="flex min-w-0 flex-1 items-center gap-3 px-4 py-3 text-left"
        onClick={() => setEditing(true)}
      >
        <StatusDot tone={meta().dotTone} ping={isRunning} />
        <div class="min-w-0 flex-1">
          <div class="truncate text-sm font-medium text-zinc-900 dark:text-zinc-100">
            {props.entry.description || (
              <span class="italic text-zinc-400">(no description)</span>
            )}
          </div>
          <div class="mt-0.5 flex items-center gap-2 text-[11px] text-zinc-500 dark:text-zinc-400">
            <Show when={props.projectName}>
              <Pill>{props.projectName}</Pill>
            </Show>
            <Show when={props.entry.billable}>
              <Pill tone="emerald">Billable</Pill>
            </Show>
            <Pill tone={meta().tone}>{meta().text}</Pill>
          </div>
        </div>
        <span class="font-mono tabular-nums text-sm font-medium text-zinc-700 dark:text-zinc-300">
          {formatDuration(entryDurationSecs(props.entry.start_at, props.entry.end_at))}
        </span>
      </button>

      <Show when={!isRunning}>
        <button
          type="button"
          class="shrink-0 px-2 py-3 text-zinc-400 transition hover:text-indigo-600 disabled:opacity-40 dark:hover:text-indigo-300"
          onClick={handleRestart}
          disabled={restarting()}
          aria-label="Restart timer with these details"
          title="Restart timer with these details"
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
            <path d="M3 10a7 7 0 1 1 2.05 4.95" />
            <path d="M3 15v-5h5" />
          </svg>
        </button>
      </Show>
      <span class="select-none px-2 py-3 text-xs text-zinc-300 dark:text-zinc-600">
        ›
      </span>

      <Show when={editing()}>
        <EditEntryDialog
          entry={props.entry}
          onClose={() => setEditing(false)}
          onSaved={() => {
            setEditing(false);
            props.onChange?.();
          }}
        />
      </Show>
    </li>
  );
}
