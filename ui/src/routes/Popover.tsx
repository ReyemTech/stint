import { For, Show, createResource, createSignal, onCleanup } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { api } from "~/api";
import Duration from "~/components/Duration";
import StartAtPicker, { type StartAtValue } from "~/components/StartAtPicker";
import Button from "~/components/ui/Button";
import ProjectTaskPicker from "~/components/ui/ProjectTaskPicker";
import SectionLabel from "~/components/ui/SectionLabel";
import StatusDot from "~/components/ui/StatusDot";
import Toggle from "~/components/ui/Toggle";
import { sumCompletedEntrySeconds } from "~/lib/entryFormat";
import { openSolidtime } from "~/lib/openSolidtime";
import { useUpdateBanner } from "~/lib/updateBanner";
import { useTimerStore } from "~/stores/timer";

export default function Popover() {
  const timer = useTimerStore();
  const [description, setDescription] = createSignal("");
  const [projectId, setProjectId] = createSignal<string | null>(null);
  const [taskId, setTaskId] = createSignal<string | null>(null);
  const [billable, setBillable] = createSignal(false);
  const [startAt, setStartAt] = createSignal<StartAtValue>(null);
  const [entries, { refetch: refetchEntries }] = createResource(
    () => api.listToday(),
    { initialValue: [] },
  );
  const [projects, { refetch: refetchProjects }] = createResource(
    () => api.listProjects(),
    { initialValue: [] },
  );
  // All tasks across projects, eager-loaded once. The combined picker
  // groups by project_id itself.
  const [tasks, { refetch: refetchTasks }] = createResource(
    () => api.listTasks(null),
    { initialValue: [] },
  );
  const unlistenEntries = listen("entries:changed", () => refetchEntries());
  // projects:changed fires after refresh_reference_data finishes (Sync now,
  // refresh_projects). Refetch so the combined picker reflects server-side
  // adds, edits, archives, and deletions immediately.
  const unlistenProjects = listen("projects:changed", () => {
    refetchProjects();
    refetchTasks();
  });
  onCleanup(() => {
    unlistenEntries.then((fn) => fn()).catch(() => {});
    unlistenProjects.then((fn) => fn()).catch(() => {});
  });

  const totalSeconds = () =>
    timer.elapsedSecs() + sumCompletedEntrySeconds(entries() ?? []);
  const updateInfo = useUpdateBanner();

  async function openMain() {
    await invoke("show_main_window");
  }

  async function openSettings() {
    await invoke("show_main_window");
    await emit("navigate", "/settings");
  }

  return (
    <div class="h-screen w-screen p-2">
      <div class="flex h-full flex-col overflow-hidden rounded-2xl border border-black/[0.06] bg-white text-zinc-900 shadow-[0_18px_40px_-12px_rgb(0_0_0/0.35),0_4px_16px_-4px_rgb(0_0_0/0.18)] dark:border-white/[0.06] dark:bg-zinc-900 dark:text-zinc-50">
        <header class="border-b border-black/[0.05] px-5 py-4 dark:border-white/[0.04]">
          <div class="flex items-center justify-between">
            <SectionLabel>{timer.running() ? "Tracking" : "Today"}</SectionLabel>
            <Show
              when={timer.running()}
              fallback={
                <SectionLabel>{(entries() ?? []).length} entries</SectionLabel>
              }
            >
              <span class="inline-flex items-center gap-1.5 text-[10px] font-medium uppercase tracking-[0.06em] text-emerald-600 dark:text-emerald-400">
                <StatusDot tone="emerald" ping />
                Live
              </span>
            </Show>
          </div>
          <div class="mt-2 text-[34px] font-semibold leading-none tracking-tight tabular-nums">
            <Duration seconds={totalSeconds()} />
          </div>
          <Show when={timer.running()}>
            {(t) => (
              <div class="mt-1.5 truncate text-[13px] text-zinc-500 dark:text-zinc-400">
                {t().description}
              </div>
            )}
          </Show>
        </header>

        <div class="flex-1 overflow-y-auto px-5 py-4">
          <Show
            when={timer.running()}
            fallback={
              <form
                class="space-y-2.5"
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
                      // Clear the task so the next start doesn't silently
                      // inherit it. Keep the project — the old single-picker
                      // popover preserved project across starts and users
                      // rely on that for back-to-back same-project work.
                      setTaskId(null);
                    });
                }}
              >
                <input
                  class="w-full rounded-lg border border-zinc-200 bg-zinc-50/50 px-3 py-2 text-[13px] outline-none ring-0 transition placeholder:text-zinc-400 focus:border-indigo-400 focus:bg-white focus:shadow-[0_0_0_3px_rgb(99_102_241/0.12)] dark:border-zinc-700 dark:bg-zinc-800/40 dark:focus:bg-zinc-800 dark:focus:border-indigo-400"
                  placeholder="What are you working on?"
                  autofocus
                  value={description()}
                  onInput={(e) => setDescription(e.currentTarget.value)}
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
                      projects={projects() ?? []}
                      tasks={tasks() ?? []}
                      placeholder="No project"
                      size="sm"
                    />
                  </div>
                  <Toggle
                    label="Billable"
                    size="sm"
                    checked={billable()}
                    onChange={setBillable}
                  />
                </div>
                <Button type="submit" block disabled={!description().trim()}>
                  Start timer
                </Button>
              </form>
            }
          >
            <Button variant="danger" block onClick={() => timer.stop()}>
              Stop timer
            </Button>
          </Show>

          <Show when={(entries() ?? []).length > 0}>
            <div class="mt-5">
              <div class="mb-2">
                <SectionLabel>Recent</SectionLabel>
              </div>
              <ul class="space-y-0.5">
                <For each={(entries() ?? []).slice(0, 4)}>
                  {(e) => {
                    const s = new Date(e.start_at).getTime();
                    const f = e.end_at ? new Date(e.end_at).getTime() : Date.now();
                    const secs = Math.max(0, Math.floor((f - s) / 1000));
                    const tone =
                      e.sync_state === "synced" ? "emerald" : "amber";
                    return (
                      <li class="flex items-center justify-between gap-2 rounded-md px-2 py-1.5 text-[12px] transition hover:bg-zinc-50 dark:hover:bg-zinc-800/50">
                        <span class="flex min-w-0 items-center gap-1.5">
                          <StatusDot tone={tone} size="xs" />
                          <span class="truncate text-zinc-700 dark:text-zinc-200">
                            {e.description || (
                              <span class="italic text-zinc-400">(no description)</span>
                            )}
                          </span>
                        </span>
                        <span class="font-mono tabular-nums text-zinc-500 dark:text-zinc-400">
                          <Duration seconds={secs} />
                        </span>
                      </li>
                    );
                  }}
                </For>
              </ul>
            </div>
          </Show>
        </div>

        <Show when={updateInfo()?.available}>
          <button
            class="border-t border-emerald-200 bg-emerald-50 px-5 py-1.5 text-center text-[11px] text-emerald-700 transition hover:bg-emerald-100 dark:border-emerald-900 dark:bg-emerald-950 dark:text-emerald-300 dark:hover:bg-emerald-900/40"
            onClick={openSettings}
          >
            Update available: v{updateInfo()!.latest_version} → open Settings
          </button>
        </Show>
        <footer class="flex items-center justify-between border-t border-black/[0.05] px-5 py-2.5 text-[11px] dark:border-white/[0.04]">
          <button
            class="text-zinc-500 transition hover:text-zinc-900 dark:text-zinc-400 dark:hover:text-zinc-100"
            onClick={openMain}
          >
            Open Stint →
          </button>
          <button
            class="text-zinc-500 transition hover:text-zinc-900 dark:text-zinc-400 dark:hover:text-zinc-100"
            onClick={openSettings}
            aria-label="Settings"
            title="Settings"
          >
            <svg
              class="h-3.5 w-3.5"
              viewBox="0 0 20 20"
              fill="currentColor"
              aria-hidden="true"
            >
              <path
                fill-rule="evenodd"
                d="M8.34 1.804A1 1 0 0 1 9.32 1h1.36a1 1 0 0 1 .98.804l.295 1.473c.497.144.965.347 1.396.6l1.25-.834a1 1 0 0 1 1.262.125l.962.962a1 1 0 0 1 .125 1.262l-.834 1.25c.253.431.456.899.6 1.396l1.473.294a1 1 0 0 1 .804.98v1.361a1 1 0 0 1-.804.98l-1.473.295a6.95 6.95 0 0 1-.6 1.396l.834 1.25a1 1 0 0 1-.125 1.262l-.962.962a1 1 0 0 1-1.262.125l-1.25-.834a6.95 6.95 0 0 1-1.396.6l-.294 1.473a1 1 0 0 1-.98.804H9.32a1 1 0 0 1-.98-.804l-.295-1.473a6.95 6.95 0 0 1-1.396-.6l-1.25.834a1 1 0 0 1-1.262-.125l-.962-.962a1 1 0 0 1-.125-1.262l.834-1.25a6.95 6.95 0 0 1-.6-1.396l-1.473-.294A1 1 0 0 1 1 10.68V9.32a1 1 0 0 1 .804-.98l1.473-.295c.144-.497.347-.965.6-1.396l-.834-1.25a1 1 0 0 1 .125-1.262l.962-.962a1 1 0 0 1 1.262-.125l1.25.834a6.95 6.95 0 0 1 1.396-.6l.294-1.473ZM10 13a3 3 0 1 0 0-6 3 3 0 0 0 0 6Z"
                clip-rule="evenodd"
              />
            </svg>
          </button>
          <button
            class="text-zinc-500 transition hover:text-zinc-900 dark:text-zinc-400 dark:hover:text-zinc-100"
            onClick={() => openSolidtime()}
          >
            Solidtime ↗
          </button>
        </footer>
      </div>
    </div>
  );
}
