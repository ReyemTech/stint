import { Show } from "solid-js";

/**
 * Pill-shaped switch with a leading colored dot. Used for boolean flags
 * (Billable, etc.) across timer cards, popover, and entry editors.
 */
export default function Toggle(props: {
  label: string;
  checked: boolean;
  onChange: (next: boolean) => void | Promise<void>;
  /** Visual tone for the "on" state. Default: emerald. */
  tone?: "emerald" | "indigo";
  size?: "sm" | "md";
  disabled?: boolean;
}) {
  const tone = () => props.tone ?? "emerald";
  const size = () => props.size ?? "md";
  return (
    <button
      type="button"
      role="switch"
      aria-checked={props.checked}
      disabled={props.disabled}
      class="inline-flex shrink-0 items-center gap-1.5 rounded-lg border font-medium transition disabled:cursor-not-allowed disabled:opacity-50"
      classList={{
        "px-2.5 py-1 text-[11px]": size() === "sm",
        "px-3 py-1.5 text-xs": size() === "md",
        "border-emerald-300 bg-emerald-50 text-emerald-700 dark:border-emerald-900 dark:bg-emerald-950/40 dark:text-emerald-300":
          props.checked && tone() === "emerald",
        "border-indigo-300 bg-indigo-50 text-indigo-700 dark:border-indigo-900 dark:bg-indigo-950/40 dark:text-indigo-300":
          props.checked && tone() === "indigo",
        "border-zinc-200 bg-white text-zinc-500 hover:text-zinc-700 dark:border-zinc-700 dark:bg-zinc-800/40 dark:text-zinc-400 dark:hover:text-zinc-200":
          !props.checked,
      }}
      onClick={() => props.onChange(!props.checked)}
    >
      <Show
        when={props.checked}
        fallback={
          <span class="h-1.5 w-1.5 rounded-full bg-zinc-300 dark:bg-zinc-600" />
        }
      >
        <span
          class="h-1.5 w-1.5 rounded-full"
          classList={{
            "bg-emerald-500": tone() === "emerald",
            "bg-indigo-500": tone() === "indigo",
          }}
        />
      </Show>
      {props.label}
    </button>
  );
}
