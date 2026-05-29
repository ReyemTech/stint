import { Show, createEffect, createSignal, onMount } from "solid-js";
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
  taskName?: string;
  isFirst?: boolean;
  /// When true, scroll this row into view + briefly highlight it (driven
  /// by `?entry=<local_uuid>` in the URL — Spotlight deep-link taps).
  focused?: boolean;
  /// Fires after any save or delete in the dialog. Callers refetch here.
  onChange?: () => void;
}) {
  const [editing, setEditing] = createSignal(false);
  const [restarting, setRestarting] = createSignal(false);
  // Holds a temporary "just focused" flag — drives a yellow ring for ~2.5s
  // after a deep-link tap so the user can see which row matched.
  const [pulse, setPulse] = createSignal(false);
  let rowEl: HTMLLIElement | undefined;
  const isRunning = !props.entry.end_at;
  const meta = () => entrySyncMeta(props.entry.sync_state, isRunning);

  function applyFocusHighlight() {
    if (!props.focused || !rowEl) return;
    // Defer scroll until layout settles after the route transition.
    requestAnimationFrame(() => {
      rowEl?.scrollIntoView({ behavior: "smooth", block: "center" });
    });
    setPulse(true);
    setTimeout(() => setPulse(false), 2500);
  }

  onMount(applyFocusHighlight);
  createEffect(() => {
    // Re-trigger when props.focused flips true on an existing row (e.g.
    // the user taps a different Spotlight result while the view is mounted).
    if (props.focused) applyFocusHighlight();
  });

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
      ref={rowEl}
      class="flex items-center transition hover:bg-zinc-50 dark:hover:bg-zinc-800/40"
      classList={{
        "border-t border-black/[0.04] dark:border-white/[0.04]": !props.isFirst,
        "ring-2 ring-amber-400 ring-inset bg-amber-50 dark:bg-amber-900/20":
          pulse(),
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
            <Show when={props.taskName}>
              <Pill tone="indigo">{props.taskName}</Pill>
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
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
          >
            <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
            <path d="M3 3v5h5" />
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
