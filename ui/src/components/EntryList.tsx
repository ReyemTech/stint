import { For, Show, createMemo, createResource } from "solid-js";
import { api } from "~/api";
import type { Entry } from "~/types";
import EntryRow from "./EntryRow";

export default function EntryList(props: {
  entries: Entry[];
  onChange?: () => void;
  onDelete?: (id: string) => void;
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
      fallback={<p class="py-4 text-center text-sm text-zinc-500">No entries.</p>}
    >
      <ul>
        <For each={props.entries}>
          {(e) => (
            <EntryRow
              entry={e}
              projectName={projectName()(e.project_id)}
              onChange={props.onChange}
              onDelete={props.onDelete}
            />
          )}
        </For>
      </ul>
    </Show>
  );
}
