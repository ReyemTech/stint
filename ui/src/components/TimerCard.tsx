import { Show, createSignal } from "solid-js";
import Duration from "./Duration";
import { useTimerStore } from "~/stores/timer";

export default function TimerCard() {
  const timer = useTimerStore();
  const [description, setDescription] = createSignal("");

  return (
    <div class="rounded-xl border border-zinc-200 bg-white p-4 shadow-sm dark:border-zinc-800 dark:bg-zinc-900">
      <Show
        when={timer.running()}
        fallback={
          <div class="flex items-center gap-2">
            <input
              class="flex-1 rounded border border-zinc-300 px-2 py-1 text-sm dark:border-zinc-700 dark:bg-zinc-800"
              placeholder="What are you working on?"
              value={description()}
              onInput={(e) => setDescription(e.currentTarget.value)}
            />
            <button
              class="rounded bg-zinc-900 px-3 py-1 text-sm font-semibold text-white dark:bg-white dark:text-zinc-900 disabled:opacity-50"
              disabled={!description().trim()}
              onClick={() =>
                timer.start(description().trim()).then(() => setDescription(""))
              }
            >
              Start
            </button>
          </div>
        }
      >
        {(t) => (
          <div>
            <div class="flex items-baseline justify-between">
              <span class="text-xs uppercase tracking-wide text-zinc-500">
                Tracking
              </span>
              <span class="text-xs text-green-600">● Live</span>
            </div>
            <div class="mt-1 text-3xl font-semibold tabular-nums">
              <Duration seconds={timer.elapsedSecs()} />
            </div>
            <div class="mt-1 text-sm text-zinc-500">{t().description}</div>
            <button
              class="mt-3 w-full rounded bg-zinc-900 py-1.5 text-sm font-semibold text-white dark:bg-white dark:text-zinc-900"
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
