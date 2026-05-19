export type DotTone = "emerald" | "amber" | "red" | "zinc" | "indigo";

const FILL: Record<DotTone, string> = {
  emerald: "bg-emerald-500",
  amber: "bg-amber-500",
  red: "bg-red-500",
  zinc: "bg-zinc-300 dark:bg-zinc-600",
  indigo: "bg-indigo-500",
};

const PING: Record<DotTone, string> = {
  emerald: "bg-emerald-400",
  amber: "bg-amber-400",
  red: "bg-red-400",
  zinc: "bg-zinc-300",
  indigo: "bg-indigo-400",
};

/**
 * A small colored dot used to encode state across the UI: sync state on
 * entry rows, "Live" indicator on timers, sync badge in the header.
 *
 * - `size` controls the diameter
 * - `ping` adds the animated ring used for live/active state
 */
export default function StatusDot(props: {
  tone: DotTone;
  size?: "xs" | "sm" | "md";
  ping?: boolean;
  class?: string;
}) {
  const size = () => props.size ?? "sm";
  const dim = () =>
    size() === "xs" ? "h-1 w-1" : size() === "sm" ? "h-1.5 w-1.5" : "h-2 w-2";

  return (
    <span class={`relative inline-flex shrink-0 ${dim()} ${props.class ?? ""}`}>
      {props.ping && (
        <span
          class={`absolute inline-flex h-full w-full animate-ping rounded-full opacity-75 ${PING[props.tone]}`}
        />
      )}
      <span
        class={`relative inline-flex ${dim()} rounded-full ${FILL[props.tone]}`}
      />
    </span>
  );
}
