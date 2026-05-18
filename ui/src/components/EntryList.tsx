import { For, Show } from "solid-js";
import EntryRow from "./EntryRow";
import type { Entry } from "~/types";

export default function EntryList(props: {
  entries: Entry[];
  onDelete?: (id: string) => void;
}) {
  return (
    <Show
      when={props.entries.length > 0}
      fallback={
        <p class="py-4 text-center text-sm text-zinc-500">No entries.</p>
      }
    >
      <ul class="divide-y divide-zinc-100 dark:divide-zinc-800">
        <For each={props.entries}>
          {(e) => <EntryRow entry={e} onDelete={props.onDelete} />}
        </For>
      </ul>
    </Show>
  );
}
