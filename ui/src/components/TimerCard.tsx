import { For, Show, createResource, createSignal } from "solid-js";
import { api } from "~/api";
import Duration from "./Duration";
import { useTimerStore } from "~/stores/timer";

export default function TimerCard() {
  const timer = useTimerStore();
  const [description, setDescription] = createSignal("");
  const [projectId, setProjectId] = createSignal<string>("");
  const [billable, setBillable] = createSignal(false);
  const [projects] = createResource(() => api.listProjects(), {
    initialValue: [],
  });
  const projectList = () => projects() ?? [];

  return (
    <div class="rounded-xl border border-zinc-200 bg-white p-4 shadow-sm dark:border-zinc-800 dark:bg-zinc-900">
      <Show
        when={timer.running()}
        fallback={
          <form
            class="space-y-2"
            onSubmit={(e) => {
              e.preventDefault();
              const d = description().trim();
              if (!d) return;
              timer
                .start(d, projectId() || undefined, billable())
                .then(() => setDescription(""));
            }}
          >
            <input
              class="w-full rounded border border-zinc-300 px-2 py-1.5 text-sm dark:border-zinc-700 dark:bg-zinc-800"
              placeholder="What are you working on?"
              value={description()}
              onInput={(e) => setDescription(e.currentTarget.value)}
              autofocus
            />
            <div class="flex items-center gap-2">
              <select
                class="flex-1 rounded border border-zinc-300 px-2 py-1.5 text-sm dark:border-zinc-700 dark:bg-zinc-800"
                value={projectId()}
                onChange={(e) => setProjectId(e.currentTarget.value)}
              >
                <option value="">No project</option>
                <For each={projectList()}>
                  {(p) => <option value={p.id}>{p.name}</option>}
                </For>
              </select>
              <label class="flex shrink-0 items-center gap-1 text-xs text-zinc-600 dark:text-zinc-300">
                <input
                  type="checkbox"
                  checked={billable()}
                  onChange={(e) => setBillable(e.currentTarget.checked)}
                />
                Billable
              </label>
              <button
                type="submit"
                class="rounded bg-zinc-900 px-3 py-1.5 text-sm font-semibold text-white disabled:opacity-50 dark:bg-white dark:text-zinc-900"
                disabled={!description().trim()}
              >
                Start
              </button>
            </div>
          </form>
        }
      >
        {(t) => (
          <div>
            <div class="flex items-baseline justify-between">
              <span class="text-xs uppercase tracking-wide text-zinc-500">Tracking</span>
              <span class="text-xs text-green-600">● Live</span>
            </div>
            <div class="mt-1 text-3xl font-semibold tabular-nums">
              <Duration seconds={timer.elapsedSecs()} />
            </div>
            <div class="mt-1 text-sm text-zinc-500">{t().description}</div>

            <div class="mt-3 flex items-center gap-2">
              <select
                class="flex-1 rounded border border-zinc-300 px-2 py-1 text-xs dark:border-zinc-700 dark:bg-zinc-800"
                value={t().project_id ?? ""}
                onChange={async (e) => {
                  const v = e.currentTarget.value;
                  await api.setEntryProject(t().local_uuid, v || null);
                  await timer.refresh();
                }}
              >
                <option value="">No project</option>
                <For each={projectList()}>
                  {(p) => <option value={p.id}>{p.name}</option>}
                </For>
              </select>
              <label class="flex shrink-0 items-center gap-1 text-xs text-zinc-600 dark:text-zinc-300">
                <input
                  type="checkbox"
                  checked={t().billable}
                  onChange={async (e) => {
                    await api.setEntryBillable(t().local_uuid, e.currentTarget.checked);
                    await timer.refresh();
                  }}
                />
                Billable
              </label>
            </div>

            <button
              class="mt-3 w-full rounded bg-red-600 py-1.5 text-sm font-semibold text-white"
              onClick={() => timer.stop()}
            >
              Stop
            </button>
          </div>
        )}
      </Show>
    </div>
  );
}
