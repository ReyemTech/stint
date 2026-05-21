import type { Project } from "~/types";

export type PickerOption = {
  id: string;
  name: string;
  clientName: string | null;
};

/**
 * Build sorted picker options:
 *   1. Drop archived projects.
 *   2. Projects WITH a client come first, ordered by client name then project name.
 *   3. Projects WITHOUT a client come at the bottom, ordered by project name.
 *
 * The "No project" placeholder option is appended at the very top by callers
 * that pass allowNone — it isn't represented here so this helper stays a pure
 * data transform.
 */
export function buildPickerOptions(projects: Project[]): PickerOption[] {
  const out: PickerOption[] = projects
    .filter((p) => !p.archived)
    .map((p) => ({ id: p.id, name: p.name, clientName: p.client_name }));
  out.sort((a, b) => {
    const aHas = a.clientName != null;
    const bHas = b.clientName != null;
    if (aHas !== bHas) return aHas ? -1 : 1;
    if (aHas && bHas) {
      const byClient = a.clientName!.localeCompare(b.clientName!);
      if (byClient !== 0) return byClient;
    }
    return a.name.localeCompare(b.name);
  });
  return out;
}
