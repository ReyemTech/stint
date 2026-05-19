import type { JSX } from "solid-js";

/** Uppercase, wide-tracked section label. Used above timer state, lists, etc. */
export default function SectionLabel(props: { children: JSX.Element }) {
  return (
    <span class="text-[10px] font-semibold uppercase tracking-[0.08em] text-zinc-400 dark:text-zinc-500">
      {props.children}
    </span>
  );
}
