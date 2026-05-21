import { Combobox } from "@kobalte/core/combobox";
import { Show, createMemo } from "solid-js";
import { buildPickerOptions, type PickerOption } from "~/lib/projectPickerSort";
import type { Project } from "~/types";

const NO_PROJECT: PickerOption = {
  id: "",
  name: "No project",
  clientName: null,
};

export default function ProjectPicker(props: {
  value: string | null;
  onChange: (id: string | null) => void;
  projects: Project[];
  placeholder?: string;
  allowNone?: boolean;
  size?: "sm" | "md";
}) {
  const options = createMemo<PickerOption[]>(() => {
    const live = buildPickerOptions(props.projects);
    return props.allowNone === false ? live : [NO_PROJECT, ...live];
  });

  const selected = createMemo<PickerOption | null>(() => {
    // When value is null, surface the placeholder rather than the
    // synthetic NO_PROJECT row (which has id="" and would otherwise
    // match an "absent" value and pre-fill the input with "No project").
    if (props.value == null) return null;
    return options().find((o) => o.id === props.value) ?? null;
  });

  const sizeClass = () =>
    props.size === "sm" ? "px-2.5 py-1.5 text-[12px]" : "px-3 py-1.5 text-sm";

  return (
    <Combobox<PickerOption>
      options={options()}
      optionValue="id"
      optionLabel="name"
      optionTextValue={(o) =>
        o.clientName ? `${o.name} ${o.clientName}` : o.name
      }
      value={selected()}
      onChange={(v) => props.onChange(v?.id ? v.id : null)}
      placeholder={props.placeholder ?? "Select project…"}
      itemComponent={(p) => (
        <Combobox.Item
          item={p.item}
          class="flex cursor-pointer items-center justify-between gap-2 rounded px-2 py-1.5 text-sm outline-none data-[highlighted]:bg-zinc-100 dark:data-[highlighted]:bg-zinc-800"
        >
          <Combobox.ItemLabel>{p.item.rawValue.name}</Combobox.ItemLabel>
          <Show when={p.item.rawValue.clientName}>
            <span class="text-[11px] text-zinc-400">
              {p.item.rawValue.clientName}
            </span>
          </Show>
        </Combobox.Item>
      )}
    >
      <Combobox.Control
        class={`flex w-full items-center gap-1 rounded-lg border border-zinc-200 bg-zinc-50/50 outline-none transition focus-within:border-indigo-400 focus-within:bg-white dark:border-zinc-700 dark:bg-zinc-800/40 dark:focus-within:bg-zinc-800 ${sizeClass()}`}
      >
        <Combobox.Input class="flex-1 bg-transparent outline-none placeholder:text-zinc-400" />
        <Combobox.Trigger
          aria-label="Open project list"
          class="text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300"
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
