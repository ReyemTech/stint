import { Popover } from "@kobalte/core/popover";
import {
  For,
  Show,
  createEffect,
  createMemo,
  createSignal,
  on,
} from "solid-js";
import { buildPickerOptions, type PickerOption } from "~/lib/projectPickerSort";
import type { Project, Task } from "~/types";

/**
 * Single combined picker for project + task selection. Replaces the
 * ProjectPicker + TaskPicker pairing wherever both are needed (popover
 * start form, TimerCard start form, TimerCard live entry, EditEntryDialog).
 *
 * The simpler ProjectPicker stays in use for Settings (default project) and
 * CalendarSection (calendar default project) — they only need a project,
 * no tasks.
 *
 * Layout: tree. "(No project)" sentinel at top, projects rendered as bold
 * rows with a chevron when they have tasks (no chevron for childless
 * projects). Project rows are selectable (selecting one yields
 * project-only, task_id=null).
 *
 * Search: smart filter. Typing matches projects AND tasks. Matching tasks
 * auto-expand their parent project; if the parent's name doesn't match the
 * query the parent is shown dimmed (still clickable for project-only).
 * Search-driven expansions don't persist after the query clears.
 *
 * Default expansion: all collapsed. The project containing the currently
 * selected task auto-expands when the dropdown opens.
 *
 * Keyboard: ↑↓ traverses visible rows; Enter selects; Esc closes; Right
 * expands a collapsed project; Left collapses an expanded project (or
 * jumps from a task back to its parent).
 */

export type ProjectTaskValue = {
  projectId: string | null;
  taskId: string | null;
};

type Row =
  | { kind: "none"; key: string }
  | {
      kind: "project";
      key: string;
      project: PickerOption;
      dim: boolean;
      hasTasks: boolean;
    }
  | { kind: "task"; key: string; project: PickerOption; task: Task };

interface Props {
  value: ProjectTaskValue;
  onChange: (v: ProjectTaskValue) => void | Promise<void>;
  projects: Project[];
  /** All tasks across all projects. The component groups by task.project_id. */
  tasks: Task[];
  placeholder?: string;
  size?: "sm" | "md";
  disabled?: boolean;
}

export default function ProjectTaskPicker(props: Props) {
  const [open, setOpen] = createSignal(false);
  const [query, setQuery] = createSignal("");
  // User-driven expansions only — survives query changes. Persists for
  // the lifetime of the dropdown instance.
  const [userExpanded, setUserExpanded] = createSignal<Set<string>>(new Set());
  const [highlightIdx, setHighlightIdx] = createSignal(0);

  // Group tasks by project_id (active tasks only) and produce a stable accessor.
  const tasksByProject = createMemo(() => {
    const m = new Map<string, Task[]>();
    for (const t of props.tasks) {
      if (t.done) continue;
      const list = m.get(t.project_id);
      if (list) list.push(t);
      else m.set(t.project_id, [t]);
    }
    for (const list of m.values()) {
      list.sort((a, b) => a.name.localeCompare(b.name));
    }
    return m;
  });

  const projectOptions = createMemo(() => buildPickerOptions(props.projects));

  // Projects auto-expanded purely because the active search query has
  // matching tasks under them. Recomputed from the query; not stored in
  // userExpanded, so clearing the query collapses them back automatically.
  const searchExpansions = createMemo<Set<string>>(() => {
    const q = query().trim().toLowerCase();
    if (!q) return new Set();
    const out = new Set<string>();
    for (const p of projectOptions()) {
      const projMatch = p.name.toLowerCase().includes(q);
      const projTasks = tasksByProject().get(p.id) ?? [];
      const hasMatchingTask = projTasks.some((t) =>
        t.name.toLowerCase().includes(q),
      );
      if (projMatch || hasMatchingTask) out.add(p.id);
    }
    return out;
  });

  // The effective expanded set is the union of user-driven and search-
  // driven expansions. Searching shows more without losing user state;
  // clearing search reverts to user state alone.
  const effectiveExpanded = createMemo<Set<string>>(() => {
    const u = userExpanded();
    const s = searchExpansions();
    if (s.size === 0) return u;
    const ns = new Set(u);
    for (const id of s) ns.add(id);
    return ns;
  });

  // Visible rows: respects search query AND effective expansion state.
  const rows = createMemo<Row[]>(() => {
    const q = query().trim().toLowerCase();
    const exp = effectiveExpanded();
    const out: Row[] = [{ kind: "none", key: "__none__" }];

    for (const p of projectOptions()) {
      const projMatch = !q || p.name.toLowerCase().includes(q);
      const projTasks = tasksByProject().get(p.id) ?? [];
      const matchingTasks = q
        ? projTasks.filter((t) => t.name.toLowerCase().includes(q))
        : projTasks;
      const hasTasks = projTasks.length > 0;

      if (q) {
        // Search mode: include project if its name matches OR it has any
        // matching task. When the project's own name doesn't match, dim
        // it to signal "the children matched, but you can still pick the
        // project itself if that's what you want."
        if (!projMatch && matchingTasks.length === 0) continue;
        out.push({
          kind: "project",
          key: `p:${p.id}`,
          project: p,
          dim: !projMatch,
          hasTasks,
        });
        for (const t of matchingTasks) {
          out.push({
            kind: "task",
            key: `t:${t.solidtime_id}`,
            project: p,
            task: t,
          });
        }
      } else {
        // Normal mode: project always visible; tasks only if expanded.
        out.push({
          kind: "project",
          key: `p:${p.id}`,
          project: p,
          dim: false,
          hasTasks,
        });
        if (exp.has(p.id)) {
          for (const t of projTasks) {
            out.push({
              kind: "task",
              key: `t:${t.solidtime_id}`,
              project: p,
              task: t,
            });
          }
        }
      }
    }
    return out;
  });

  // When the dropdown opens, auto-expand the project that owns the
  // currently selected task so the user can see the current value
  // without first having to expand by hand. This counts as a user
  // expansion since the user effectively selected this previously.
  createEffect(
    on(open, (isOpen) => {
      if (!isOpen) return;
      const v = props.value;
      if (v.taskId && v.projectId) {
        setUserExpanded((s) => {
          if (s.has(v.projectId!)) return s;
          const ns = new Set(s);
          ns.add(v.projectId!);
          return ns;
        });
      }
      // Reset query + highlight on every open so the user starts clean.
      setQuery("");
      setHighlightIdx(0);
    }),
  );

  // Reset highlight on query change to avoid pointing at a filtered-out row.
  createEffect(
    on(query, () => {
      setHighlightIdx(0);
    }),
  );

  // Resolve the displayed label for the current value.
  const valueLabel = createMemo(() => {
    const v = props.value;
    if (v.projectId == null) return null;
    const p = projectOptions().find((o) => o.id === v.projectId);
    if (!p) return null;
    if (v.taskId == null) return p.name;
    const t = props.tasks.find((x) => x.solidtime_id === v.taskId);
    return t ? `${p.name} / ${t.name}` : p.name;
  });

  function commitRow(row: Row) {
    switch (row.kind) {
      case "none":
        void props.onChange({ projectId: null, taskId: null });
        break;
      case "project":
        void props.onChange({ projectId: row.project.id, taskId: null });
        break;
      case "task":
        void props.onChange({
          projectId: row.project.id,
          taskId: row.task.solidtime_id,
        });
        break;
    }
    setOpen(false);
  }

  function toggleUserExpand(projectId: string) {
    setUserExpanded((s) => {
      const ns = new Set(s);
      if (ns.has(projectId)) ns.delete(projectId);
      else ns.add(projectId);
      return ns;
    });
  }

  function onKeyDown(e: KeyboardEvent) {
    const list = rows();
    const idx = highlightIdx();
    switch (e.key) {
      case "ArrowDown": {
        e.preventDefault();
        if (list.length > 0) {
          setHighlightIdx(Math.min(list.length - 1, idx + 1));
        }
        break;
      }
      case "ArrowUp": {
        e.preventDefault();
        setHighlightIdx(Math.max(0, idx - 1));
        break;
      }
      case "ArrowRight": {
        const row = list[idx];
        if (
          row?.kind === "project" &&
          row.hasTasks &&
          !effectiveExpanded().has(row.project.id)
        ) {
          e.preventDefault();
          toggleUserExpand(row.project.id);
        }
        break;
      }
      case "ArrowLeft": {
        const row = list[idx];
        if (
          row?.kind === "project" &&
          effectiveExpanded().has(row.project.id)
        ) {
          e.preventDefault();
          toggleUserExpand(row.project.id);
        } else if (row?.kind === "task") {
          e.preventDefault();
          const parentIdx = list.findIndex(
            (r) => r.kind === "project" && r.project.id === row.project.id,
          );
          if (parentIdx >= 0) setHighlightIdx(parentIdx);
        }
        break;
      }
      case "Enter": {
        e.preventDefault();
        const row = list[idx];
        if (row) commitRow(row);
        break;
      }
      case "Escape": {
        e.preventDefault();
        setOpen(false);
        break;
      }
    }
  }

  const sizeClass = () =>
    props.size === "sm" ? "px-2.5 py-1.5 text-[12px]" : "px-3 py-1.5 text-sm";

  return (
    <Popover open={open()} onOpenChange={setOpen} placement="bottom-start">
      <Popover.Trigger
        aria-label="Open project or task list"
        disabled={props.disabled}
        class={`flex w-full items-center gap-1 rounded-lg border border-zinc-200 bg-zinc-50/50 text-left outline-none transition focus-visible:border-indigo-400 focus-visible:bg-white disabled:opacity-50 dark:border-zinc-700 dark:bg-zinc-800/40 dark:focus-visible:bg-zinc-800 ${sizeClass()}`}
      >
        <span class="flex-1 truncate">
          <Show
            when={valueLabel()}
            fallback={
              <span class="text-zinc-400">
                {props.placeholder ?? "Select project or task…"}
              </span>
            }
          >
            {(label) => <span>{label()}</span>}
          </Show>
        </span>
        <span
          aria-hidden
          class="text-zinc-400 transition hover:text-zinc-600 dark:hover:text-zinc-300"
        >
          ▾
        </span>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content
          class="z-50 mt-1 w-72 rounded-lg border border-black/[0.08] bg-white p-1 shadow-lg outline-none dark:border-white/[0.08] dark:bg-zinc-950"
          onOpenAutoFocus={(e) => e.preventDefault()}
        >
          <input
            type="text"
            autofocus
            value={query()}
            placeholder="Search projects + tasks…"
            class="mb-1 w-full rounded-md border border-transparent bg-zinc-100/70 px-2 py-1.5 text-sm outline-none placeholder:text-zinc-400 focus:border-indigo-400 focus:bg-white dark:bg-zinc-800/50 dark:focus:bg-zinc-900"
            onInput={(e) => setQuery(e.currentTarget.value)}
            onKeyDown={onKeyDown}
          />
          <ul
            role="listbox"
            class="max-h-72 space-y-0.5 overflow-y-auto"
            aria-label="Project or task"
          >
            <For each={rows()}>
              {(row, i) => (
                <RowItem
                  row={row}
                  highlighted={i() === highlightIdx()}
                  isExpanded={
                    row.kind === "project" &&
                    effectiveExpanded().has(row.project.id)
                  }
                  onSelect={() => commitRow(row)}
                  onToggle={() => {
                    if (row.kind === "project") toggleUserExpand(row.project.id);
                  }}
                  onHover={() => setHighlightIdx(i())}
                />
              )}
            </For>
            <Show when={rows().length === 1 && query().trim()}>
              <li class="px-2 py-3 text-center text-xs text-zinc-400">
                No projects or tasks match "{query()}"
              </li>
            </Show>
          </ul>
        </Popover.Content>
      </Popover.Portal>
    </Popover>
  );
}

function RowItem(props: {
  row: Row;
  highlighted: boolean;
  isExpanded: boolean;
  onSelect: () => void;
  onToggle: () => void;
  onHover: () => void;
}) {
  const baseClass = () =>
    `flex cursor-pointer items-center gap-1 rounded px-2 py-1.5 text-sm outline-none ${
      props.highlighted ? "bg-zinc-100 dark:bg-zinc-800" : ""
    }`;

  return (
    <li
      role="option"
      aria-selected={props.highlighted}
      class={baseClass()}
      onMouseEnter={props.onHover}
      onClick={(e) => {
        // Clicks on the chevron-zone (data-chevron) only toggle expansion,
        // not select. Clicks anywhere else in the row select.
        const target = e.target as HTMLElement;
        if (target.closest("[data-chevron]")) return;
        props.onSelect();
      }}
    >
      {(() => {
        switch (props.row.kind) {
          case "none":
            return (
              <span class="flex-1 text-zinc-500 dark:text-zinc-400">
                No project
              </span>
            );
          case "project": {
            const row = props.row;
            return (
              <>
                <Show
                  when={row.hasTasks}
                  fallback={
                    <span
                      class="flex h-6 w-6 shrink-0 items-center justify-center text-zinc-400 dark:text-zinc-500"
                      aria-hidden
                    >
                      •
                    </span>
                  }
                >
                  <button
                    type="button"
                    data-chevron
                    aria-label={props.isExpanded ? "Collapse" : "Expand"}
                    tabIndex={-1}
                    class="-ml-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded text-zinc-500 transition hover:bg-zinc-200 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-700 dark:hover:text-zinc-100"
                    onClick={(e) => {
                      e.stopPropagation();
                      props.onToggle();
                    }}
                  >
                    <svg
                      width="10"
                      height="10"
                      viewBox="0 0 10 10"
                      fill="none"
                      class="transition-transform"
                      style={{
                        transform: props.isExpanded
                          ? "rotate(90deg)"
                          : "rotate(0deg)",
                      }}
                    >
                      <path
                        d="M3 1.5L7 5L3 8.5"
                        stroke="currentColor"
                        stroke-width="1.5"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                      />
                    </svg>
                  </button>
                </Show>
                <span
                  class={
                    "flex-1 truncate font-medium " +
                    (row.dim
                      ? "text-zinc-400 dark:text-zinc-500"
                      : "text-zinc-900 dark:text-zinc-100")
                  }
                >
                  {row.project.name}
                </span>
                <Show when={row.project.clientName}>
                  <span class="text-[11px] text-zinc-400">
                    {row.project.clientName}
                  </span>
                </Show>
              </>
            );
          }
          case "task":
            return (
              <>
                <span class="w-6" aria-hidden />
                <span
                  class="flex w-4 shrink-0 items-center text-xs text-zinc-400 dark:text-zinc-500"
                  aria-hidden
                >
                  –
                </span>
                <span class="flex-1 truncate text-xs text-zinc-600 dark:text-zinc-400">
                  {props.row.task.name}
                </span>
              </>
            );
        }
      })()}
    </li>
  );
}
