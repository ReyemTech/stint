import { For, Show, createResource, createSignal, onCleanup } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { api } from "~/api";
import Duration from "~/components/Duration";
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
      <div
        class="flex h-full flex-col overflow-hidden rounded-2xl border border-black/[0.06] bg-white text-zinc-900 shadow-[0_18px_40px_-12px_rgb(0_0_0/0.35),0_4px_16px_-4px_rgb(0_0_0/0.18)] dark:border-white/[0.06] dark:bg-zinc-900 dark:text-zinc-50"
      >
        {/* Header */}
        <header class="border-b border-black/[0.05] px-5 py-4 dark:border-white/[0.04]">
          <div class="flex items-center justify-between">
            <span class="text-[10px] font-semibold uppercase tracking-[0.08em] text-zinc-400 dark:text-zinc-500">
              {timer.running() ? "Tracking" : "Today"}
            </span>
            <Show
              when={timer.running()}
              fallback={
                <span class="text-[10px] uppercase tracking-[0.08em] text-zinc-400 dark:text-zinc-500">
                  {(entries() ?? []).length} entries
                </span>
              }
            >
              <span class="inline-flex items-center gap-1.5 text-[10px] font-medium uppercase tracking-[0.06em] text-emerald-600 dark:text-emerald-400">
                <span class="relative inline-flex h-1.5 w-1.5">
                  <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400 opacity-75" />
                  <span class="relative inline-flex h-1.5 w-1.5 rounded-full bg-emerald-500" />
                </span>
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

        {/* Body */}
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
                  <BillableToggle
                    checked={billable()}
                    onChange={setBillable}
                  />
                </div>
                <button
                  type="submit"
                  class="mt-1 w-full rounded-lg bg-zinc-900 px-3 py-2 text-[13px] font-semibold text-white shadow-sm transition hover:bg-zinc-700 active:scale-[0.99] disabled:cursor-not-allowed disabled:opacity-40 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
                  disabled={!description().trim()}
                >
                  Start timer
                </button>
              </form>
            }
          >
            <button
              class="w-full rounded-lg bg-red-500 px-3 py-2 text-[13px] font-semibold text-white shadow-sm transition hover:bg-red-600 active:scale-[0.99]"
              onClick={() => timer.stop()}
            >
              Stop timer
            </button>
          </Show>

          <Show when={(entries() ?? []).length > 0}>
            <div class="mt-5">
              <div class="mb-2 text-[10px] font-semibold uppercase tracking-[0.08em] text-zinc-400 dark:text-zinc-500">
                Recent
              </div>
              <ul class="space-y-0.5">
                <For each={(entries() ?? []).slice(0, 4)}>
                  {(e) => {
                    const s = new Date(e.start_at).getTime();
                    const f = e.end_at ? new Date(e.end_at).getTime() : Date.now();
                    const secs = Math.max(0, Math.floor((f - s) / 1000));
                    const synced = e.sync_state === "synced";
                    return (
                      <li class="flex items-center justify-between gap-2 rounded-md px-2 py-1.5 text-[12px] transition hover:bg-zinc-50 dark:hover:bg-zinc-800/50">
                        <span class="flex min-w-0 items-center gap-1.5">
                          <span
                            class="h-1 w-1 shrink-0 rounded-full"
                            classList={{
                              "bg-emerald-500": synced,
                              "bg-amber-500": !synced,
                            }}
                          />
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

        {/* Footer */}
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

function BillableToggle(props: {
  checked: boolean;
  onChange: (next: boolean) => void;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={props.checked}
      class="inline-flex shrink-0 items-center gap-1.5 rounded-lg border px-2.5 py-1.5 text-[11px] font-medium transition"
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
