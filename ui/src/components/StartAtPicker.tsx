import { For, Show, createSignal } from "solid-js";
import {
  isoMinutesAgo,
  parseLocalHHMMTodayOrYesterday,
  startAtLabel,
} from "~/lib/startAt";

type Preset = { label: string; minutesAgo: number };

const PRESETS: Preset[] = [
  { label: "5 min", minutesAgo: 5 },
  { label: "15 min", minutesAgo: 15 },
  { label: "30 min", minutesAgo: 30 },
  { label: "1 hour", minutesAgo: 60 },
];

/// `null` ↔ "Start now" (no override). Otherwise an ISO 8601 UTC timestamp.
export type StartAtValue = string | null;

export default function StartAtPicker(props: {
  value: StartAtValue;
  onChange: (v: StartAtValue) => void;
}) {
  const [open, setOpen] = createSignal(false);
  const [custom, setCustom] = createSignal("");

  function pickPreset(minutesAgo: number) {
    props.onChange(isoMinutesAgo(minutesAgo));
  }

  function pickCustom() {
    const parsed = parseLocalHHMMTodayOrYesterday(custom());
    if (parsed) props.onChange(parsed);
  }

  function clear() {
    props.onChange(null);
  }

  return (
    <div class="text-xs">
      <button
        type="button"
        class="text-zinc-500 underline-offset-2 hover:text-zinc-900 hover:underline dark:text-zinc-400 dark:hover:text-zinc-100"
        onClick={() => setOpen((v) => !v)}
        aria-label="Open start-time picker"
      >
        {startAtLabel(props.value)}
        <span class="ml-1 text-zinc-400">▾</span>
      </button>
      <Show when={open()}>
        <div class="mt-2 flex flex-wrap items-center gap-1.5">
          <button
            type="button"
            class="rounded-full bg-zinc-100 px-2.5 py-1 text-[11px] hover:bg-zinc-200 dark:bg-zinc-800 dark:hover:bg-zinc-700"
            onClick={clear}
          >
            Now
          </button>
          <For each={PRESETS}>
            {(p) => (
              <button
                type="button"
                class="rounded-full bg-zinc-100 px-2.5 py-1 text-[11px] hover:bg-zinc-200 dark:bg-zinc-800 dark:hover:bg-zinc-700"
                onClick={() => pickPreset(p.minutesAgo)}
              >
                {p.label} ago
              </button>
            )}
          </For>
          <input
            type="time"
            class="rounded border border-zinc-200 bg-white px-1.5 py-0.5 text-[11px] dark:border-zinc-700 dark:bg-zinc-950"
            value={custom()}
            onInput={(e) => setCustom(e.currentTarget.value)}
            onBlur={pickCustom}
            onKeyDown={(e) => {
              if (e.key === "Enter") pickCustom();
            }}
          />
        </div>
      </Show>
    </div>
  );
}
