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
              timer.start(d, projectId() || undefined, billable()).then(() => {
                setDescription("");
                setBillable(false);
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
            <div class="flex items-center gap-2">
              <select
                class="min-w-0 flex-1 rounded-lg border border-zinc-200 bg-zinc-50/50 px-3 py-1.5 text-sm outline-none transition focus:border-indigo-400 focus:bg-white dark:border-zinc-700 dark:bg-zinc-800/40 dark:focus:bg-zinc-800"
                value={projectId()}
                onChange={(e) => setProjectId(e.currentTarget.value)}
              >
                <option value="">No project</option>
                <For each={projectList()}>
                  {(p) => <option value={p.id}>{p.name}</option>}
                </For>
              </select>
              <BillableToggle checked={billable()} onChange={setBillable} />
              <button
                type="submit"
                class="rounded-lg bg-zinc-900 px-4 py-1.5 text-sm font-semibold text-white shadow-sm transition hover:bg-zinc-700 active:scale-[0.99] disabled:cursor-not-allowed disabled:opacity-40 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
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
            <div class="flex items-center justify-between">
              <span class="text-[10px] font-semibold uppercase tracking-[0.08em] text-zinc-400 dark:text-zinc-500">
                Tracking
              </span>
              <span class="inline-flex items-center gap-1.5 text-[10px] font-medium uppercase tracking-[0.06em] text-emerald-600 dark:text-emerald-400">
                <span class="relative inline-flex h-1.5 w-1.5">
                  <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400 opacity-75" />
                  <span class="relative inline-flex h-1.5 w-1.5 rounded-full bg-emerald-500" />
                </span>
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
              <select
                class="min-w-0 flex-1 rounded-lg border border-zinc-200 bg-zinc-50/50 px-3 py-1.5 text-xs outline-none transition focus:border-indigo-400 focus:bg-white dark:border-zinc-700 dark:bg-zinc-800/40 dark:focus:bg-zinc-800"
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
              <BillableToggle
                checked={t().billable}
                onChange={async (next) => {
                  await api.setEntryBillable(t().local_uuid, next);
                  await timer.refresh();
                }}
              />
              <button
                class="rounded-lg bg-red-500 px-4 py-1.5 text-sm font-semibold text-white shadow-sm transition hover:bg-red-600 active:scale-[0.99]"
                onClick={() => timer.stop()}
              >
                Stop
              </button>
            </div>
          </div>
        )}
      </Show>
    </div>
  );
}

function BillableToggle(props: {
  checked: boolean;
  onChange: (next: boolean) => void | Promise<void>;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={props.checked}
      class="inline-flex shrink-0 items-center gap-1.5 rounded-lg border px-3 py-1.5 text-xs font-medium transition"
      classList={{
        "border-emerald-300 bg-emerald-50 text-emerald-700 dark:border-emerald-900 dark:bg-emerald-950/40 dark:text-emerald-300":
          props.checked,
        "border-zinc-200 bg-white text-zinc-500 hover:text-zinc-700 dark:border-zinc-700 dark:bg-zinc-800/40 dark:text-zinc-400 dark:hover:text-zinc-200":
          !props.checked,
      }}
      onClick={() => props.onChange(!props.checked)}
    >
      <span
        class="h-1.5 w-1.5 rounded-full"
        classList={{
          "bg-emerald-500": props.checked,
          "bg-zinc-300 dark:bg-zinc-600": !props.checked,
        }}
      />
      Billable
    </button>
  );
}
