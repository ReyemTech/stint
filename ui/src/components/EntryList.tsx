import { For, Show, createMemo, createResource, onCleanup } from "solid-js";
import { listen } from "@tauri-apps/api/event";
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
  const [projects, { refetch: refetchProjects }] = createResource(
    () => api.listProjects(),
    { initialValue: [] },
  );
  const projectName = createMemo(() => {
    const map = new Map<string, string>();
    for (const p of projects() ?? []) map.set(p.id, p.name);
    return (id: string | null | undefined) => (id ? map.get(id) : undefined);
  });

  // Resolve task names by fetching the locally-cached task list for the
  // visible day. Only triggers when at least one entry references a task —
  // saves an IPC for the common "no tasks yet" case. Tracks every entry's
  // task_id (any signal change refires the resource), but the actual fetch
  // returns all tasks the local DB knows about in one call.
  const needsTasks = createMemo(() =>
    props.entries.some((e) => e.task_id != null),
  );
  const [tasks, { refetch: refetchTasks }] = createResource(
    needsTasks,
    async (need) => (need ? await api.listTasks() : []),
    { initialValue: [] },
  );

  // projects:changed fires after refresh_reference_data succeeds. A
  // Solidtime-side rename or deletion otherwise leaves stale labels on
  // historical entries until the list is unmounted and remounted.
  const unlistenProjects = listen("projects:changed", () => {
    refetchProjects();
    refetchTasks();
  });
  onCleanup(() => {
    unlistenProjects.then((fn) => fn()).catch(() => {});
  });
  const taskName = createMemo(() => {
    const map = new Map<string, string>();
    for (const t of tasks() ?? []) map.set(t.solidtime_id, t.name);
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
              taskName={taskName()(e.task_id)}
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
