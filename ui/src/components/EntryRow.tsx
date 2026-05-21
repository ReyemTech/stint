import { For, Show, createResource, createSignal } from "solid-js";
import { api } from "~/api";
import { entryDurationSecs, entrySyncMeta } from "~/lib/entryFormat";
import type { Entry } from "~/types";
import { formatDuration } from "./Duration";
import Pill from "./ui/Pill";
import StatusDot from "./ui/StatusDot";
import Toggle from "./ui/Toggle";
import Button from "./ui/Button";

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
  const meta = () => entrySyncMeta(props.entry.sync_state, isRunning);

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
          <div class="flex items-end gap-3">
            <div class="flex-1">
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
            <Toggle
              label="Billable"
              checked={props.entry.billable}
              onChange={changeBillable}
            />
          </div>
          <div class="flex justify-end pt-1">
            <Button
              variant="secondary"
              size="sm"
              onClick={() => props.onDelete?.(props.entry.local_uuid)}
            >
              Delete entry
            </Button>
          </div>
        </div>
      </Show>
    </li>
  );
}
