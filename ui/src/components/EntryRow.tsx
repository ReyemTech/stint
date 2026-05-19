import { For, Show, createResource, createSignal } from "solid-js";
import { api } from "~/api";
import type { Entry } from "~/types";
import { formatDuration } from "./Duration";

function durationSecs(start: string, end: string | null): number {
  const s = new Date(start).getTime();
  const e = end ? new Date(end).getTime() : Date.now();
  return Math.max(0, Math.floor((e - s) / 1000));
}

export default function EntryRow(props: {
  entry: Entry;
  projectName?: string;
  onChange?: () => void;
  onDelete?: (id: string) => void;
}) {
  const [open, setOpen] = createSignal(false);
  const [desc, setDesc] = createSignal(props.entry.description);
  const [savingDesc, setSavingDesc] = createSignal(false);
  const [projects] = createResource(() => api.listProjects(), {
    initialValue: [],
  });
  const isRunning = !props.entry.end_at;

  async function saveDescription() {
    if (desc().trim() === props.entry.description.trim()) return;
    setSavingDesc(true);
    try {
      await api.updateDescription(props.entry.local_uuid, desc().trim());
      props.onChange?.();
    } finally {
      setSavingDesc(false);
    }
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
    <li class="border-b border-zinc-100 dark:border-zinc-800">
      <button
        type="button"
        class="flex w-full items-center justify-between gap-3 px-1 py-2 text-left"
        onClick={() => setOpen((v) => !v)}
      >
        <div class="min-w-0 flex-1">
          <div class="truncate text-sm">
            {props.entry.description || (
              <span class="italic text-zinc-400">(no description)</span>
            )}
          </div>
          <div class="mt-0.5 flex items-center gap-2 text-xs text-zinc-500">
            <Show when={props.projectName}>
              <span class="rounded bg-zinc-100 px-1.5 py-0.5 text-[10px] font-medium text-zinc-700 dark:bg-zinc-800 dark:text-zinc-300">
                {props.projectName}
              </span>
            </Show>
            <Show when={props.entry.billable}>
              <span class="rounded bg-emerald-50 px-1.5 py-0.5 text-[10px] font-medium text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300">
                Billable
              </span>
            </Show>
            <span
              class="rounded px-1.5 py-0.5 text-[10px] font-medium uppercase"
              classList={{
                "bg-zinc-100 text-zinc-600 dark:bg-zinc-800 dark:text-zinc-300":
                  !isRunning,
                "bg-green-100 text-green-700 dark:bg-green-950 dark:text-green-300":
                  isRunning,
              }}
            >
              {isRunning ? "Running" : props.entry.sync_state}
            </span>
          </div>
        </div>
        <span class="font-mono tabular-nums text-sm text-zinc-700 dark:text-zinc-300">
          {formatDuration(durationSecs(props.entry.start_at, props.entry.end_at))}
        </span>
        <span
          class="select-none text-xs text-zinc-400 transition-transform"
          classList={{ "rotate-90": open() }}
        >
          ›
        </span>
      </button>

      <Show when={open()}>
        <div class="grid gap-3 border-t border-zinc-100 bg-zinc-50 px-3 py-3 text-sm dark:border-zinc-800 dark:bg-zinc-950">
          <div>
            <label class="block text-[11px] font-medium text-zinc-500">
              Description
            </label>
            <input
              type="text"
              class="mt-1 w-full rounded border border-zinc-300 bg-white px-2 py-1 text-sm dark:border-zinc-700 dark:bg-zinc-900"
              value={desc()}
              onInput={(e) => setDesc(e.currentTarget.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") (e.currentTarget as HTMLInputElement).blur();
              }}
              onBlur={saveDescription}
              disabled={savingDesc()}
            />
          </div>
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-[11px] font-medium text-zinc-500">Project</label>
              <select
                class="mt-1 w-full rounded border border-zinc-300 bg-white px-2 py-1 text-sm dark:border-zinc-700 dark:bg-zinc-900"
                value={props.entry.project_id ?? ""}
                onChange={(e) => changeProject(e.currentTarget.value)}
              >
                <option value="">No project</option>
                <For each={projects() ?? []}>
                  {(p) => <option value={p.id}>{p.name}</option>}
                </For>
              </select>
            </div>
            <div class="flex items-end">
              <label class="flex items-center gap-2 text-sm text-zinc-700 dark:text-zinc-300">
                <input
                  type="checkbox"
                  checked={props.entry.billable}
                  onChange={(e) => changeBillable(e.currentTarget.checked)}
                />
                Billable
              </label>
            </div>
          </div>
          <div class="flex justify-end gap-2 pt-1">
            <button
              class="rounded border border-red-300 px-2.5 py-1 text-xs text-red-700 transition hover:bg-red-50 dark:border-red-900 dark:text-red-300 dark:hover:bg-red-950"
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
