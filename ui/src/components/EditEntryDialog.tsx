import { createMemo, createResource, createSignal, Show } from "solid-js";
import { api } from "~/api";
import { fromLocalHHMM, toLocalHHMM } from "~/lib/entryFormat";
import type { Entry } from "~/types";
import Button from "./ui/Button";
import ProjectTaskPicker from "./ui/ProjectTaskPicker";
import Toggle from "./ui/Toggle";

export default function EditEntryDialog(props: {
  entry: Entry;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [desc, setDesc] = createSignal(props.entry.description);
  const [projectId, setProjectId] = createSignal<string | null>(
    props.entry.project_id,
  );
  const [taskId, setTaskId] = createSignal<string | null>(props.entry.task_id);
  const [billable, setBillable] = createSignal(props.entry.billable);
  const startHHMMInitial = toLocalHHMM(props.entry.start_at);
  const endHHMMInitial = props.entry.end_at
    ? toLocalHHMM(props.entry.end_at)
    : "";
  const [startHHMM, setStartHHMM] = createSignal(startHHMMInitial);
  const [endHHMM, setEndHHMM] = createSignal(endHHMMInitial);
  const [err, setErr] = createSignal<string | null>(null);
  const [confirmingDelete, setConfirmingDelete] = createSignal(false);

  const [projects] = createResource(() => api.listProjects(), {
    initialValue: [],
  });
  // All tasks across all projects, eager-loaded once. The combined picker
  // groups by project_id itself.
  const [tasks] = createResource(() => api.listTasks(null), {
    initialValue: [],
  });

  const isCompleted = createMemo(() => Boolean(props.entry.end_at));

  async function save() {
    setErr(null);
    try {
      if (desc().trim() !== props.entry.description.trim()) {
        await api.updateDescription(props.entry.local_uuid, desc().trim());
      }
      if (projectId() !== props.entry.project_id) {
        await api.setEntryProject(props.entry.local_uuid, projectId());
      }
      if (taskId() !== props.entry.task_id) {
        await api.setEntryTask(props.entry.local_uuid, taskId());
      }
      if (billable() !== props.entry.billable) {
        await api.setEntryBillable(props.entry.local_uuid, billable());
      }
      if (
        isCompleted() &&
        (startHHMM() !== startHHMMInitial || endHHMM() !== endHHMMInitial)
      ) {
        // Only rebuild + send when the user touched the time inputs.
        // Rebuilding always zeroes seconds, so a no-op save would silently
        // truncate non-zero-second timer entries to minute precision.
        const newStart = fromLocalHHMM(props.entry.start_at, startHHMM());
        const newEnd = fromLocalHHMM(props.entry.end_at!, endHHMM());
        await api.updateEntryTimes(props.entry.local_uuid, newStart, newEnd);
      }
      props.onSaved();
      props.onClose();
    } catch (e) {
      setErr((e as { message: string }).message);
    }
  }

  async function destroy() {
    // Inline two-step confirm: first click arms, second click commits.
    // window.confirm() is silently suppressed by Tauri's WKWebView.
    if (!confirmingDelete()) {
      setConfirmingDelete(true);
      return;
    }
    try {
      await api.deleteEntry(props.entry.local_uuid);
      props.onSaved();
      props.onClose();
    } catch (e) {
      setErr((e as { message: string }).message);
      setConfirmingDelete(false);
    }
  }

  return (
    <div
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4"
      onClick={(e) => {
        if (e.target === e.currentTarget) props.onClose();
      }}
    >
      <div class="w-full max-w-md rounded-2xl border border-black/[0.06] bg-white p-5 shadow-xl dark:border-white/[0.06] dark:bg-zinc-900">
        <h2 class="mb-4 text-base font-semibold">Edit entry</h2>

        <div class="space-y-3">
          <div>
            <label class="block text-[10px] font-semibold uppercase tracking-[0.08em] text-zinc-400 dark:text-zinc-500">
              Description
            </label>
            <input
              type="text"
              class="mt-1 w-full rounded-md border border-zinc-200 bg-white px-2.5 py-1.5 text-sm outline-none focus:border-indigo-400 dark:border-zinc-700 dark:bg-zinc-950"
              value={desc()}
              onInput={(e) => setDesc(e.currentTarget.value)}
            />
          </div>

          <div>
            <label class="block text-[10px] font-semibold uppercase tracking-[0.08em] text-zinc-400 dark:text-zinc-500">
              Project / task
            </label>
            <div class="mt-1">
              <ProjectTaskPicker
                value={{ projectId: projectId(), taskId: taskId() }}
                onChange={(v) => {
                  setProjectId(v.projectId);
                  setTaskId(v.taskId);
                }}
                projects={projects() ?? []}
                tasks={tasks() ?? []}
                placeholder="No project"
                size="sm"
              />
            </div>
          </div>

          <Show when={isCompleted()}>
            <div class="flex items-end gap-3">
              <div class="flex-1">
                <label class="block text-[10px] font-semibold uppercase tracking-[0.08em] text-zinc-400 dark:text-zinc-500">
                  Start
                </label>
                <input
                  type="time"
                  class="mt-1 w-full rounded-md border border-zinc-200 bg-white px-2.5 py-1.5 text-sm outline-none focus:border-indigo-400 dark:border-zinc-700 dark:bg-zinc-950"
                  value={startHHMM()}
                  onInput={(e) => setStartHHMM(e.currentTarget.value)}
                />
              </div>
              <div class="flex-1">
                <label class="block text-[10px] font-semibold uppercase tracking-[0.08em] text-zinc-400 dark:text-zinc-500">
                  End
                </label>
                <input
                  type="time"
                  class="mt-1 w-full rounded-md border border-zinc-200 bg-white px-2.5 py-1.5 text-sm outline-none focus:border-indigo-400 dark:border-zinc-700 dark:bg-zinc-950"
                  value={endHHMM()}
                  onInput={(e) => setEndHHMM(e.currentTarget.value)}
                />
              </div>
            </div>
          </Show>

          <div>
            <Toggle
              label="Billable"
              checked={billable()}
              onChange={setBillable}
            />
          </div>
        </div>

        <Show when={err()}>
          <p class="mt-3 text-xs text-red-600 dark:text-red-400">{err()}</p>
        </Show>

        <div class="mt-5 flex items-center justify-between gap-2">
          <Show
            when={confirmingDelete()}
            fallback={
              <>
                <Button variant="ghost" size="sm" onClick={destroy}>
                  Delete
                </Button>
                <div class="flex gap-2">
                  <Button variant="ghost" onClick={props.onClose}>
                    Cancel
                  </Button>
                  <Button onClick={save}>Save</Button>
                </div>
              </>
            }
          >
            <span class="text-sm text-red-600 dark:text-red-400">
              Delete this entry?
            </span>
            <div class="flex gap-2">
              <Button
                variant="ghost"
                onClick={() => {
                  setConfirmingDelete(false);
                }}
              >
                Cancel
              </Button>
              <Button variant="danger" onClick={destroy}>
                Yes, delete
              </Button>
            </div>
          </Show>
        </div>
      </div>
    </div>
  );
}
