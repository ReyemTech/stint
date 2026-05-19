import { For, Show, createResource, createSignal } from "solid-js";
import { api } from "~/api";
import type { Entry } from "~/types";
import { formatDuration } from "./Duration";

function durationSecs(start: string, end: string | null): number {
  const s = new Date(start).getTime();
  const e = end ? new Date(end).getTime() : Date.now();
  return Math.max(0, Math.floor((e - s) / 1000));
}

function syncLabel(state: Entry["sync_state"]): { text: string; tone: string } {
  switch (state) {
    case "synced":
      return { text: "Synced", tone: "emerald" };
    case "dirty":
      return { text: "Edited", tone: "amber" };
    case "pending_create":
      return { text: "Pending", tone: "amber" };
    case "pending_delete":
      return { text: "Deleting", tone: "red" };
  }
}

export default function EntryRow(props: {
  entry: Entry;
  projectName?: string;
  isFirst?: boolean;
  onChange?: () => void;
  onDelete?: (id: string) => void;
}) {
  const [open, setOpen] = createSignal(false);
  const [desc, setDesc] = createSignal(props.entry.description);
  const [projects] = createResource(() => api.listProjects(), {
    initialValue: [],
  });
  const isRunning = !props.entry.end_at;
  const sync = () => syncLabel(props.entry.sync_state);

  async function saveDescription() {
    if (desc().trim() === props.entry.description.trim()) return;
    await api.updateDescription(props.entry.local_uuid, desc().trim());
    props.onChange?.();
  }

  async function changeProject(projectId: string) {
    await api.setEntryProject(props.entry.local_uuid, projectId || null);
    props.onChange?.();
  }

  async function changeBillable(billable: boolean) {
    await api.setEntryBillable(props.entry.local_uuid, billable);
    props.onChange?.();
  }

  return (
    <li
      classList={{
        "border-t border-black/[0.04] dark:border-white/[0.04]": !props.isFirst,
      }}
    >
      <button
        type="button"
        class="flex w-full items-center gap-3 px-4 py-3 text-left transition hover:bg-zinc-50 dark:hover:bg-zinc-800/40"
        onClick={() => setOpen((v) => !v)}
      >
        <span
          class="h-2 w-2 shrink-0 rounded-full"
          classList={{
            "bg-emerald-500": isRunning || props.entry.sync_state === "synced",
            "bg-amber-500":
              !isRunning &&
              (props.entry.sync_state === "pending_create" ||
                props.entry.sync_state === "dirty"),
            "bg-red-500": props.entry.sync_state === "pending_delete",
          }}
        />
        <div class="min-w-0 flex-1">
          <div class="truncate text-sm font-medium text-zinc-900 dark:text-zinc-100">
            {props.entry.description || (
              <span class="italic text-zinc-400">(no description)</span>
            )}
          </div>
          <div class="mt-0.5 flex items-center gap-2 text-[11px] text-zinc-500 dark:text-zinc-400">
            <Show when={props.projectName}>
              <span class="rounded bg-zinc-100 px-1.5 py-0.5 font-medium text-zinc-600 dark:bg-zinc-800 dark:text-zinc-300">
                {props.projectName}
              </span>
            </Show>
            <Show when={props.entry.billable}>
              <span class="rounded bg-emerald-50 px-1.5 py-0.5 font-medium text-emerald-700 dark:bg-emerald-950/60 dark:text-emerald-300">
                Billable
              </span>
            </Show>
            <Show when={!isRunning}>
              <span
                class="rounded px-1.5 py-0.5 font-medium"
                classList={{
                  "bg-emerald-50 text-emerald-700 dark:bg-emerald-950/40 dark:text-emerald-300":
                    sync().tone === "emerald",
                  "bg-amber-50 text-amber-700 dark:bg-amber-950/40 dark:text-amber-300":
                    sync().tone === "amber",
                  "bg-red-50 text-red-700 dark:bg-red-950/40 dark:text-red-300":
                    sync().tone === "red",
                }}
              >
                {sync().text}
              </span>
            </Show>
            <Show when={isRunning}>
              <span class="rounded bg-emerald-50 px-1.5 py-0.5 font-medium text-emerald-700 dark:bg-emerald-950/40 dark:text-emerald-300">
                Running
              </span>
            </Show>
          </div>
        </div>
        <span class="font-mono tabular-nums text-sm font-medium text-zinc-700 dark:text-zinc-300">
          {formatDuration(durationSecs(props.entry.start_at, props.entry.end_at))}
        </span>
        <span
          class="select-none text-xs text-zinc-300 transition-transform dark:text-zinc-600"
          classList={{ "rotate-90": open() }}
        >
          ›
        </span>
      </button>

      <Show when={open()}>
        <div class="grid gap-3 border-t border-black/[0.04] bg-zinc-50/60 px-4 py-3 text-sm dark:border-white/[0.04] dark:bg-zinc-950/40">
          <div>
            <label class="block text-[10px] font-semibold uppercase tracking-[0.08em] text-zinc-400 dark:text-zinc-500">
              Description
            </label>
            <input
              type="text"
              class="mt-1 w-full rounded-md border border-zinc-200 bg-white px-2.5 py-1.5 text-sm outline-none transition focus:border-indigo-400 focus:shadow-[0_0_0_3px_rgb(99_102_241/0.12)] dark:border-zinc-700 dark:bg-zinc-900"
              value={desc()}
              onInput={(e) => setDesc(e.currentTarget.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") (e.currentTarget as HTMLInputElement).blur();
              }}
              onBlur={saveDescription}
            />
          </div>
          <div class="grid grid-cols-[1fr_auto] gap-3">
            <div>
              <label class="block text-[10px] font-semibold uppercase tracking-[0.08em] text-zinc-400 dark:text-zinc-500">
                Project
              </label>
              <select
                class="mt-1 w-full rounded-md border border-zinc-200 bg-white px-2.5 py-1.5 text-sm outline-none transition focus:border-indigo-400 dark:border-zinc-700 dark:bg-zinc-900"
                value={props.entry.project_id ?? ""}
                onChange={(e) => changeProject(e.currentTarget.value)}
              >
                <option value="">No project</option>
                <For each={projects() ?? []}>
                  {(p) => <option value={p.id}>{p.name}</option>}
                </For>
              </select>
            </div>
            <div class="flex items-end pb-px">
              <label class="inline-flex items-center gap-2 text-xs text-zinc-700 dark:text-zinc-300">
                <input
                  type="checkbox"
                  class="accent-indigo-500"
                  checked={props.entry.billable}
                  onChange={(e) => changeBillable(e.currentTarget.checked)}
                />
                Billable
              </label>
            </div>
          </div>
          <div class="flex justify-end pt-1">
            <button
              class="rounded-md border border-red-200 bg-white px-2.5 py-1 text-xs font-medium text-red-700 transition hover:bg-red-50 dark:border-red-900/50 dark:bg-zinc-900 dark:text-red-300 dark:hover:bg-red-950/30"
              onClick={() => props.onDelete?.(props.entry.local_uuid)}
            >
              Delete entry
            </button>
          </div>
        </div>
      </Show>
    </li>
  );
}
