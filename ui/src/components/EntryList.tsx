import { For, Show, createMemo, createResource } from "solid-js";
import { api } from "~/api";
import type { Entry } from "~/types";
import EntryRow from "./EntryRow";

export default function EntryList(props: {
  entries: Entry[];
  /// When set, the matching entry row scrolls into view + briefly highlights.
  /// Driven by `?entry=<local_uuid>` in the route (Spotlight deep-link taps).
  focusUuid?: string;
  /// Fires after any save or delete in a row's edit dialog. Callers refetch here.
  onChange?: () => void;
}) {
  const [projects] = createResource(() => api.listProjects(), {
    initialValue: [],
  });
  const projectName = createMemo(() => {
    const map = new Map<string, string>();
    for (const p of projects() ?? []) map.set(p.id, p.name);
    return (id: string | null | undefined) => (id ? map.get(id) : undefined);
  });

  return (
    <Show
      when={props.entries.length > 0}
      fallback={
        <p class="py-12 text-center text-sm text-zinc-400 dark:text-zinc-500">
          No entries yet today.
        </p>
      }
    >
      <ul>
        <For each={props.entries}>
          {(e, i) => (
            <EntryRow
              entry={e}
              projectName={projectName()(e.project_id)}
              isFirst={i() === 0}
              focused={props.focusUuid === e.local_uuid}
              onChange={props.onChange}
            />
          )}
        </For>
      </ul>
    </Show>
  );
}
