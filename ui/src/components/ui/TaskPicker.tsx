import { Combobox } from "@kobalte/core/combobox";
import { createMemo } from "solid-js";
import type { Task } from "~/types";

type Option = { id: string; name: string };

const NO_TASK: Option = { id: "", name: "No task" };

/// Mirrors ProjectPicker. The two material differences:
///   * `projectSelected` — when false, the input is disabled and shows
///     a hint placeholder. The picker stays in the DOM so layout doesn't
///     reflow when a project is added/removed mid-edit.
///   * `tasks` are pre-scoped by the parent to the selected project; we
///     don't filter here.
export default function TaskPicker(props: {
  value: string | null;
  onChange: (id: string | null) => void;
  tasks: Task[];
  /// When false, the input is disabled and the placeholder shifts to a
  /// hint that the user must pick a project first.
  projectSelected: boolean;
  placeholder?: string;
  size?: "sm" | "md";
}) {
  const options = createMemo<Option[]>(() => {
    const live = props.tasks.map<Option>((t) => ({
      id: t.solidtime_id,
      name: t.name,
    }));
    return [NO_TASK, ...live];
  });

  const selected = createMemo<Option | null>(() => {
    // Mirror ProjectPicker: null value must NOT match the synthetic
    // NO_TASK row (id=""), otherwise the input pre-fills with "No task"
    // instead of the placeholder.
    if (props.value == null) return null;
    return options().find((o) => o.id === props.value) ?? null;
  });

  const sizeClass = () =>
    props.size === "sm" ? "px-2.5 py-1.5 text-[12px]" : "px-3 py-1.5 text-sm";

  const placeholder = () =>
    props.projectSelected
      ? (props.placeholder ?? "Select task…")
      : "Pick a project first";

  return (
    <Combobox<Option>
      options={options()}
      optionValue="id"
      optionLabel="name"
      optionTextValue={(o) => o.name}
      value={selected()}
      onChange={(v) => props.onChange(v?.id ? v.id : null)}
      placeholder={placeholder()}
      disabled={!props.projectSelected}
      itemComponent={(p) => (
        <Combobox.Item
          item={p.item}
          class="flex cursor-pointer items-center justify-between gap-2 rounded px-2 py-1.5 text-sm outline-none data-[highlighted]:bg-zinc-100 dark:data-[highlighted]:bg-zinc-800"
        >
          <Combobox.ItemLabel>{p.item.rawValue.name}</Combobox.ItemLabel>
        </Combobox.Item>
      )}
    >
      <Combobox.Control
        class={`flex w-full items-center gap-1 rounded-lg border border-zinc-200 bg-zinc-50/50 outline-none transition focus-within:border-indigo-400 focus-within:bg-white disabled:opacity-50 dark:border-zinc-700 dark:bg-zinc-800/40 dark:focus-within:bg-zinc-800 ${sizeClass()}`}
      >
        <Combobox.Input class="flex-1 bg-transparent outline-none placeholder:text-zinc-400 disabled:cursor-not-allowed" />
        <Combobox.Trigger
          aria-label="Open task list"
          class="text-zinc-400 hover:text-zinc-600 disabled:opacity-50 dark:hover:text-zinc-300"
        >
          ▾
        </Combobox.Trigger>
      </Combobox.Control>
      <Combobox.Portal>
        <Combobox.Content class="z-50 max-h-72 overflow-y-auto rounded-lg border border-black/[0.08] bg-white p-1 shadow-lg dark:border-white/[0.08] dark:bg-zinc-950">
          <Combobox.Listbox class="space-y-0.5" />
        </Combobox.Content>
      </Combobox.Portal>
    </Combobox>
  );
}
