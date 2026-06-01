import { Show, createResource, createSignal } from "solid-js";
import { api } from "~/api";
import Duration from "./Duration";
import StartAtPicker, { type StartAtValue } from "./StartAtPicker";
import Button from "./ui/Button";
import ProjectTaskPicker from "./ui/ProjectTaskPicker";
import SectionLabel from "./ui/SectionLabel";
import StatusDot from "./ui/StatusDot";
import Toggle from "./ui/Toggle";
import { useTimerStore } from "~/stores/timer";

export default function TimerCard() {
  const timer = useTimerStore();
  const [description, setDescription] = createSignal("");
  const [projectId, setProjectId] = createSignal<string | null>(null);
  const [taskId, setTaskId] = createSignal<string | null>(null);
  const [billable, setBillable] = createSignal(false);
  const [startAt, setStartAt] = createSignal<StartAtValue>(null);
  const [projects] = createResource(() => api.listProjects(), {
    initialValue: [],
  });
  const projectList = () => projects() ?? [];

  // All tasks across all projects, eager-loaded once. The combined picker
  // groups by project_id itself, so we don't have to refetch per selection.
  const [tasks] = createResource(() => api.listTasks(null), {
    initialValue: [],
  });

  /// Apply a project+task change to a running entry. Always clears the
  /// task first so a queued patch can't carry a task_id from the old
  /// project, then sets the new project, then sets the new task.
  async function applyLiveChange(args: {
    localUuid: string;
    hadTask: boolean;
    nextProjectId: string | null;
    nextTaskId: string | null;
  }) {
    if (args.hadTask) {
      await api.setEntryTask(args.localUuid, null);
    }
    await api.setEntryProject(args.localUuid, args.nextProjectId);
    if (args.nextTaskId) {
      await api.setEntryTask(args.localUuid, args.nextTaskId);
    }
    await timer.refresh();
  }

  return (
    <div class="rounded-2xl border border-black/[0.06] bg-white p-5 shadow-sm dark:border-white/[0.06] dark:bg-zinc-900">
      <Show
        when={timer.running()}
        fallback={
          <form
            class="space-y-3"
            onSubmit={(e) => {
              e.preventDefault();
              const d = description().trim();
              if (!d) return;
              timer
                .start(
                  d,
                  projectId() ?? undefined,
                  taskId() ?? undefined,
                  billable(),
                  startAt() ?? undefined,
                )
                .then(() => {
                  setDescription("");
                  setBillable(false);
                  setStartAt(null);
                  setTaskId(null);
                  setProjectId(null);
                });
            }}
          >
            <input
              class="w-full rounded-lg border border-zinc-200 bg-zinc-50/50 px-3 py-2 text-sm outline-none transition placeholder:text-zinc-400 focus:border-indigo-400 focus:bg-white focus:shadow-[0_0_0_3px_rgb(99_102_241/0.12)] dark:border-zinc-700 dark:bg-zinc-800/40 dark:focus:bg-zinc-800 dark:focus:border-indigo-400"
              placeholder="What are you working on?"
              value={description()}
              onInput={(e) => setDescription(e.currentTarget.value)}
              autofocus
            />
            <StartAtPicker value={startAt()} onChange={setStartAt} />
            <div class="flex items-center gap-2">
              <div class="min-w-0 flex-1">
                <ProjectTaskPicker
                  value={{ projectId: projectId(), taskId: taskId() }}
                  onChange={(v) => {
                    setProjectId(v.projectId);
                    setTaskId(v.taskId);
                  }}
                  projects={projectList()}
                  tasks={tasks() ?? []}
                  placeholder="Project / task"
                />
              </div>
              <Toggle label="Billable" checked={billable()} onChange={setBillable} />
              <Button type="submit" disabled={!description().trim()}>
                Start
              </Button>
            </div>
          </form>
        }
      >
        {(t) => (
          <div>
            <div class="flex items-center justify-between">
              <SectionLabel>Tracking</SectionLabel>
              <span class="inline-flex items-center gap-1.5 text-[10px] font-medium uppercase tracking-[0.06em] text-emerald-600 dark:text-emerald-400">
                <StatusDot tone="emerald" ping />
                Live
              </span>
            </div>
            <div class="mt-2 text-[40px] font-semibold leading-none tracking-tight tabular-nums">
              <Duration seconds={timer.elapsedSecs()} />
            </div>
            <div class="mt-1.5 truncate text-sm text-zinc-500 dark:text-zinc-400">
              {t().description}
            </div>

            <div class="mt-4 flex items-center gap-2">
              <div class="min-w-0 flex-1">
                <ProjectTaskPicker
                  value={{
                    projectId: t().project_id,
                    taskId: t().task_id,
                  }}
                  onChange={(v) =>
                    applyLiveChange({
                      localUuid: t().local_uuid,
                      hadTask: Boolean(t().task_id),
                      nextProjectId: v.projectId,
                      nextTaskId: v.taskId,
                    })
                  }
                  projects={projectList()}
                  tasks={tasks() ?? []}
                  placeholder="Project / task"
                  size="sm"
                />
              </div>
              <Toggle
                label="Billable"
                checked={t().billable}
                onChange={async (next) => {
                  await api.setEntryBillable(t().local_uuid, next);
                  await timer.refresh();
                }}
              />
              <Button variant="danger" onClick={() => timer.stop()}>
                Stop
              </Button>
            </div>
          </div>
        )}
      </Show>
    </div>
  );
}
