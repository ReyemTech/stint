import type { JSX } from "solid-js";

export type PillTone = "neutral" | "emerald" | "amber" | "red" | "indigo";

const TONES: Record<PillTone, string> = {
  neutral:
    "bg-zinc-100 text-zinc-600 dark:bg-zinc-800 dark:text-zinc-300",
  emerald:
    "bg-emerald-50 text-emerald-700 dark:bg-emerald-950/40 dark:text-emerald-300",
  amber:
    "bg-amber-50 text-amber-700 dark:bg-amber-950/40 dark:text-amber-300",
  red:
    "bg-red-50 text-red-700 dark:bg-red-950/40 dark:text-red-300",
  indigo:
    "bg-indigo-50 text-indigo-700 dark:bg-indigo-950/40 dark:text-indigo-300",
};

/** Small inline label badge — project tag, sync state, billable marker, etc. */
export default function Pill(props: {
  tone?: PillTone;
  children: JSX.Element;
}) {
  const tone = () => props.tone ?? "neutral";
  return (
    <span
      class={`rounded px-1.5 py-0.5 text-[11px] font-medium ${TONES[tone()]}`}
    >
      {props.children}
    </span>
  );
}
