import { For, Show, createResource, createSignal, onCleanup } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { api } from "~/api";
import Duration from "~/components/Duration";
import { useTimerStore } from "~/stores/timer";

export default function Popover() {
  const timer = useTimerStore();
  const [description, setDescription] = createSignal("");
  const [projectId, setProjectId] = createSignal<string>("");
  const [billable, setBillable] = createSignal(false);
  const [entries, { refetch: refetchEntries }] = createResource(
    () => api.listToday(),
    { initialValue: [] },
  );
  const refetchId = window.setInterval(() => refetchEntries(), 3000);
  onCleanup(() => window.clearInterval(refetchId));
  const [projects] = createResource(() => api.listProjects(), {
    initialValue: [],
  });

  const totalSeconds = () => {
    let total = timer.elapsedSecs();
    for (const e of entries() ?? []) {
      if (!e.end_at) continue;
      const s = new Date(e.start_at).getTime();
      const f = new Date(e.end_at).getTime();
      total += Math.max(0, Math.floor((f - s) / 1000));
    }
    return total;
  };

  async function openMain() {
    // Rust command hides the popover then shows the main window.
    await invoke("show_main_window");
  }

  return (
    <div class="flex h-full flex-col bg-white text-zinc-900 dark:bg-zinc-950 dark:text-zinc-50">
      <div class="border-b border-zinc-200 px-4 py-3 dark:border-zinc-800">
        <div class="flex items-baseline justify-between">
          <span class="text-[10px] font-semibold uppercase tracking-wider text-zinc-500">
            {timer.running() ? "Tracking" : "Today"}
          </span>
          <Show when={timer.running()}>
            <span class="flex items-center gap-1 text-[10px] text-green-600 dark:text-green-400">
              <span class="inline-block h-1.5 w-1.5 animate-pulse rounded-full bg-green-500" />
              Live
            </span>
          </Show>
        </div>
        <div class="mt-1 text-3xl font-semibold tabular-nums leading-none">
          <Duration seconds={totalSeconds()} />
        </div>
        <Show
          when={timer.running()}
          fallback={
            <div class="mt-1 text-xs text-zinc-500">
              No timer running · {entries()?.length ?? 0} entries today
            </div>
          }
        >
          {(t) => (
            <div class="mt-1 truncate text-xs text-zinc-500">{t().description}</div>
          )}
        </Show>
      </div>

      <div class="flex-1 px-4 py-3">
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
                class="w-full rounded-md border border-zinc-300 bg-zinc-50 px-3 py-2 text-sm placeholder:text-zinc-400 focus:border-zinc-900 focus:bg-white focus:outline-none dark:border-zinc-700 dark:bg-zinc-900 dark:focus:border-zinc-50 dark:focus:bg-zinc-800"
                placeholder="What are you working on?"
                autofocus
                value={description()}
                onInput={(e) => setDescription(e.currentTarget.value)}
              />
              <div class="flex items-center gap-2">
                <select
                  class="flex-1 rounded-md border border-zinc-300 bg-zinc-50 px-2 py-1.5 text-xs dark:border-zinc-700 dark:bg-zinc-900"
                  value={projectId()}
                  onChange={(e) => setProjectId(e.currentTarget.value)}
                >
                  <option value="">No project</option>
                  <For each={projects() ?? []}>
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
              </div>
              <button
                type="submit"
                class="w-full rounded-md bg-zinc-900 py-2 text-sm font-semibold text-white transition hover:bg-zinc-700 disabled:opacity-40 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-200"
                disabled={!description().trim()}
              >
                Start timer
              </button>
            </form>
          }
        >
          <button
            class="w-full rounded-md bg-red-600 py-2 text-sm font-semibold text-white transition hover:bg-red-700"
            onClick={() => timer.stop()}
          >
            Stop timer
          </button>
        </Show>

        <Show when={(entries() ?? []).length > 0}>
          <div class="mt-4">
            <div class="mb-1 text-[10px] font-semibold uppercase tracking-wider text-zinc-500">
              Recent
            </div>
            <ul class="space-y-1">
              {(entries() ?? []).slice(0, 4).map((e) => {
                const s = new Date(e.start_at).getTime();
                const f = e.end_at ? new Date(e.end_at).getTime() : Date.now();
                const secs = Math.max(0, Math.floor((f - s) / 1000));
                return (
                  <li class="flex items-center justify-between text-xs">
                    <span class="truncate pr-2 text-zinc-700 dark:text-zinc-300">
                      {e.description || "(no description)"}
                    </span>
                    <span class="font-mono tabular-nums text-zinc-500">
                      <Duration seconds={secs} />
                    </span>
                  </li>
                );
              })}
            </ul>
          </div>
        </Show>
      </div>

      <div class="border-t border-zinc-200 px-4 py-2 dark:border-zinc-800">
        <button
          class="w-full text-left text-[11px] text-zinc-500 transition hover:text-zinc-900 dark:hover:text-zinc-100"
          onClick={openMain}
        >
          Open Stint →
        </button>
      </div>
    </div>
  );
}
