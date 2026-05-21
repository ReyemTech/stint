import { Collapsible } from "@kobalte/core/collapsible";
import { JSX, Show } from "solid-js";

/**
 * Collapsible card section built on `@kobalte/core/collapsible` for keyboard
 * + ARIA handling. Same prop shape as the previous hand-rolled component:
 * title, optional hint, optional right slot, defaultOpen, children.
 */
export default function Accordion(props: {
  title: string;
  hint?: string;
  right?: JSX.Element;
  defaultOpen?: boolean;
  children: JSX.Element;
}) {
  return (
    <Collapsible
      class="rounded-2xl border border-black/[0.06] bg-white dark:border-white/[0.06] dark:bg-zinc-900"
      defaultOpen={props.defaultOpen}
    >
      <Collapsible.Trigger class="flex w-full items-center justify-between gap-3 px-6 py-4 text-left">
        <div class="min-w-0 flex-1">
          <div class="flex items-center gap-2">
            <h2 class="text-sm font-semibold uppercase tracking-wide text-zinc-500">
              {props.title}
            </h2>
            <Show when={props.right}>
              <div class="ml-auto flex items-center gap-2">{props.right}</div>
            </Show>
          </div>
          <Show when={props.hint}>
            <p class="mt-1 text-xs text-zinc-500">{props.hint}</p>
          </Show>
        </div>
        <svg
          class="h-4 w-4 shrink-0 text-zinc-400 transition-transform data-[expanded]:rotate-90"
          viewBox="0 0 20 20"
          fill="currentColor"
          aria-hidden="true"
        >
          <path
            fill-rule="evenodd"
            d="M7.21 14.77a.75.75 0 0 1 .02-1.06L11.17 10 7.23 6.29a.75.75 0 1 1 1.04-1.08l4.5 4.25a.75.75 0 0 1 0 1.08l-4.5 4.25a.75.75 0 0 1-1.06-.02Z"
            clip-rule="evenodd"
          />
        </svg>
      </Collapsible.Trigger>
      <Collapsible.Content class="border-t border-black/[0.05] px-6 pb-6 pt-5 dark:border-white/[0.04]">
        {props.children}
      </Collapsible.Content>
    </Collapsible>
  );
}
