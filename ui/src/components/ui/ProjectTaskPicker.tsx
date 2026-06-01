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
 * rows with a chevron, tasks indented underneath. Project rows are
 * selectable (selecting one yields project-only, task_id=null).
 *
 * Search: smart filter. Typing matches projects AND tasks. Matching tasks
 * auto-expand their parent project; if the parent's name doesn't match the
 * query the parent is shown dimmed (still clickable for project-only).
 *
 * Default expansion: all collapsed. The project containing the currently
 * selected task auto-expands when the dropdown opens. Any query
 * auto-expands all projects with matches.
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
  | { kind: "project"; key: string; project: PickerOption; dim: boolean }
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
  const [expanded, setExpanded] = createSignal<Set<string>>(new Set());
  const [highlightIdx, setHighlightIdx] = createSignal(0);

  // Group tasks by project_id and produce a stable accessor.
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

  // Visible rows: respects search query AND expansion state.
  const rows = createMemo<Row[]>(() => {
    const q = query().trim().toLowerCase();
    const exp = expanded();
    const out: Row[] = [{ kind: "none", key: "__none__" }];

    for (const p of projectOptions()) {
      const projMatch = !q || p.name.toLowerCase().includes(q);
      const projTasks = tasksByProject().get(p.id) ?? [];
      const matchingTasks = q
        ? projTasks.filter((t) => t.name.toLowerCase().includes(q))
        : projTasks;

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
        out.push({ kind: "project", key: `p:${p.id}`, project: p, dim: false });
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
  // without first having to expand by hand.
  createEffect(
    on(open, (isOpen) => {
      if (!isOpen) return;
      const v = props.value;
      if (v.taskId && v.projectId) {
        setExpanded((s) => {
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

  // Auto-expand any project that has matches whenever the query changes.
  // Empty query restores whatever expansion the user had explicitly set;
  // we only expand on non-empty queries.
  createEffect(
    on(query, (q) => {
      const trimmed = q.trim().toLowerCase();
      if (!trimmed) return;
      const matching = new Set<string>();
      for (const p of projectOptions()) {
        const projMatch = p.name.toLowerCase().includes(trimmed);
        const projTasks = tasksByProject().get(p.id) ?? [];
        const hasMatchingTask = projTasks.some((t) =>
          t.name.toLowerCase().includes(trimmed),
        );
        if (projMatch || hasMatchingTask) matching.add(p.id);
      }
      setExpanded((s) => {
        // Union with whatever the user had open so we don't collapse
        // them mid-search.
        const ns = new Set(s);
        for (const id of matching) ns.add(id);
        return ns;
      });
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

  function toggleExpand(projectId: string) {
    setExpanded((s) => {
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
        if (row?.kind === "project" && !expanded().has(row.project.id)) {
          e.preventDefault();
          toggleExpand(row.project.id);
        }
        break;
      }
      case "ArrowLeft": {
        const row = list[idx];
        if (row?.kind === "project" && expanded().has(row.project.id)) {
          e.preventDefault();
          toggleExpand(row.project.id);
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
                    row.kind === "project" && expanded().has(row.project.id)
                  }
                  onSelect={() => commitRow(row)}
                  onToggle={() => {
                    if (row.kind === "project") toggleExpand(row.project.id);
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
          case "project":
            return (
              <>
                <button
                  type="button"
                  data-chevron
                  aria-label={props.isExpanded ? "Collapse" : "Expand"}
                  tabIndex={-1}
                  class="flex h-4 w-4 items-center justify-center rounded text-[10px] text-zinc-400 hover:bg-zinc-200 hover:text-zinc-700 dark:hover:bg-zinc-700 dark:hover:text-zinc-200"
                  onClick={(e) => {
                    e.stopPropagation();
                    props.onToggle();
                  }}
                >
                  {props.isExpanded ? "▾" : "▸"}
                </button>
                <span
                  class={
                    "flex-1 truncate font-medium " +
                    (props.row.dim
                      ? "text-zinc-400 dark:text-zinc-500"
                      : "text-zinc-900 dark:text-zinc-100")
                  }
                >
                  {props.row.project.name}
                </span>
                <Show when={props.row.project.clientName}>
                  <span class="text-[11px] text-zinc-400">
                    {props.row.project.clientName}
                  </span>
                </Show>
              </>
            );
          case "task":
            return (
              <>
                <span class="w-4" aria-hidden />
                <span class="flex-1 truncate pl-2 text-zinc-700 dark:text-zinc-300">
                  ↳ {props.row.task.name}
                </span>
              </>
            );
        }
      })()}
    </li>
  );
}
