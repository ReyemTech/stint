import { For, Show, createResource, createSignal, onCleanup } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { api } from "~/api";
import Duration from "~/components/Duration";
import Button from "~/components/ui/Button";
import SectionLabel from "~/components/ui/SectionLabel";
import StatusDot from "~/components/ui/StatusDot";
import Toggle from "~/components/ui/Toggle";
import { openSolidtime } from "~/lib/openSolidtime";
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
  const [projects] = createResource(() => api.listProjects(), {
    initialValue: [],
  });
  const unlistenEntries = listen("entries:changed", () => refetchEntries());
  onCleanup(() => {
    unlistenEntries.then((fn) => fn()).catch(() => {});
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
    await invoke("show_main_window");
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
                    .start(d, projectId() || undefined, billable())
                    .then(() => {
                      setDescription("");
                      setBillable(false);
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
                <div class="flex items-center gap-2">
                  <select
                    class="min-w-0 flex-1 rounded-lg border border-zinc-200 bg-zinc-50/50 px-2.5 py-1.5 text-[12px] outline-none transition focus:border-indigo-400 focus:bg-white dark:border-zinc-700 dark:bg-zinc-800/40 dark:focus:bg-zinc-800"
                    value={projectId()}
                    onChange={(e) => setProjectId(e.currentTarget.value)}
                  >
                    <option value="">No project</option>
                    <For each={projects() ?? []}>
                      {(p) => <option value={p.id}>{p.name}</option>}
                    </For>
                  </select>
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

        <footer class="flex items-center justify-between border-t border-black/[0.05] px-5 py-2.5 text-[11px] dark:border-white/[0.04]">
          <button
            class="text-zinc-500 transition hover:text-zinc-900 dark:text-zinc-400 dark:hover:text-zinc-100"
            onClick={openMain}
          >
            Open Stint →
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
