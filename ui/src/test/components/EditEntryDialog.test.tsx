import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render } from "@solidjs/testing-library";

vi.mock("~/api", () => ({
  api: {
    listProjects: vi.fn().mockResolvedValue([
      { id: "p-1", name: "Tet", color: null, client_id: null, client_name: null, archived: 0 },
    ]),
    listTasks: vi.fn().mockResolvedValue([
      { solidtime_id: "t-1", project_id: "p-1", name: "Implement", done: false },
    ]),
    updateDescription: vi.fn().mockResolvedValue(undefined),
    setEntryProject: vi.fn().mockResolvedValue(undefined),
    setEntryTask: vi.fn().mockResolvedValue(undefined),
    setEntryBillable: vi.fn().mockResolvedValue(undefined),
    updateEntryTimes: vi.fn().mockResolvedValue(undefined),
    deleteEntry: vi.fn().mockResolvedValue(undefined),
  },
}));

import EditEntryDialog from "~/components/EditEntryDialog";
import { api } from "~/api";
import type { Entry } from "~/types";

const flush = () => new Promise<void>((r) => setTimeout(r, 0));

function entry(overrides: Partial<Entry> = {}): Entry {
  return {
    local_uuid: "uuid-1",
    solidtime_id: null,
    description: "deep work",
    project_id: null,
    task_id: null,
    start_at: "2026-05-20T09:00:00Z",
    end_at: "2026-05-20T09:30:00Z",
    billable: false,
    sync_state: "synced",
    source: "cli",
    ...overrides,
  };
}

beforeEach(() => {
  Object.values(api).forEach((fn) => {
    if (vi.isMockFunction(fn)) fn.mockClear();
  });
});

describe("<EditEntryDialog>", () => {
  it("renders the description, project/task picker, billable toggle, and time inputs for a completed entry", () => {
    const { getByText, getByLabelText, container } = render(() => (
      <EditEntryDialog
        entry={entry()}
        onClose={vi.fn()}
        onSaved={vi.fn()}
      />
    ));
    expect(getByText("Edit entry")).toBeDefined();
    expect(getByText("Description")).toBeDefined();
    expect(getByText("Project / task")).toBeDefined();
    expect(getByText("Start")).toBeDefined();
    expect(getByText("End")).toBeDefined();
    expect(getByLabelText("Open project or task list")).toBeDefined();
    const times = container.querySelectorAll('input[type="time"]');
    expect(times.length).toBe(2);
  });

  it("hides the start/end time inputs for a running entry", () => {
    const { queryByText } = render(() => (
      <EditEntryDialog
        entry={entry({ end_at: null })}
        onClose={vi.fn()}
        onSaved={vi.fn()}
      />
    ));
    expect(queryByText("Start")).toBeNull();
    expect(queryByText("End")).toBeNull();
  });

  it("Cancel button invokes onClose", async () => {
    const onClose = vi.fn();
    const { getByText } = render(() => (
      <EditEntryDialog
        entry={entry()}
        onClose={onClose}
        onSaved={vi.fn()}
      />
    ));
    fireEvent.click(getByText("Cancel"));
    expect(onClose).toHaveBeenCalled();
  });

  it("Save with only a description change calls api.updateDescription and onSaved+onClose", async () => {
    const onSaved = vi.fn();
    const onClose = vi.fn();
    const { container, getByText } = render(() => (
      <EditEntryDialog
        entry={entry()}
        onClose={onClose}
        onSaved={onSaved}
      />
    ));
    const descInput = container.querySelector('input[type="text"]') as HTMLInputElement;
    descInput.value = "deep work (renamed)";
    fireEvent.input(descInput);
    fireEvent.click(getByText("Save"));
    await flush();
    expect(api.updateDescription).toHaveBeenCalledWith(
      "uuid-1",
      "deep work (renamed)",
    );
    expect(onSaved).toHaveBeenCalled();
    expect(onClose).toHaveBeenCalled();
  });

  it("does not call updateEntryTimes when the user only edits metadata on a non-zero-second entry", async () => {
    // Regression: rebuilding HH:MM → ISO always zeroes seconds. Without the
    // guard, a metadata-only Save would silently shift a 09:00:42 start to
    // 09:00:00, truncating recorded duration.
    const onSaved = vi.fn();
    const onClose = vi.fn();
    const { container, getByText } = render(() => (
      <EditEntryDialog
        entry={entry({
          start_at: "2026-05-20T09:00:42Z",
          end_at: "2026-05-20T09:30:17Z",
        })}
        onClose={onClose}
        onSaved={onSaved}
      />
    ));
    const descInput = container.querySelector(
      'input[type="text"]',
    ) as HTMLInputElement;
    descInput.value = "deep work (renamed)";
    fireEvent.input(descInput);
    fireEvent.click(getByText("Save"));
    await flush();
    expect(api.updateDescription).toHaveBeenCalled();
    expect(api.updateEntryTimes).not.toHaveBeenCalled();
  });

  it("clicking the scrim (outer wrapper) invokes onClose", async () => {
    const onClose = vi.fn();
    const { container } = render(() => (
      <EditEntryDialog
        entry={entry()}
        onClose={onClose}
        onSaved={vi.fn()}
      />
    ));
    const scrim = container.querySelector(".fixed.inset-0") as HTMLDivElement;
    // Simulate a click whose target is the scrim itself.
    fireEvent.click(scrim, {});
    expect(onClose).toHaveBeenCalled();
  });

  it("Delete arms a two-step confirm; second click calls api.deleteEntry and onSaved+onClose", async () => {
    const onSaved = vi.fn();
    const onClose = vi.fn();
    const { getByText, findByText } = render(() => (
      <EditEntryDialog
        entry={entry()}
        onClose={onClose}
        onSaved={onSaved}
      />
    ));
    // First click only arms; nothing is deleted.
    fireEvent.click(getByText("Delete"));
    await flush();
    expect(api.deleteEntry).not.toHaveBeenCalled();
    expect(await findByText("Delete this entry?")).toBeDefined();
    // Second click commits.
    fireEvent.click(getByText("Yes, delete"));
    await flush();
    expect(api.deleteEntry).toHaveBeenCalledWith("uuid-1");
    expect(onSaved).toHaveBeenCalled();
    expect(onClose).toHaveBeenCalled();
  });

  it("eagerly loads all tasks for the combined project/task picker", async () => {
    render(() => (
      <EditEntryDialog
        entry={entry({ project_id: "p-1" })}
        onClose={vi.fn()}
        onSaved={vi.fn()}
      />
    ));
    await flush();
    // The combined picker fetches all tasks upfront (project_id=null) so
    // it can group them under their parents. The previous separate
    // TaskPicker fetched per-project on demand.
    expect(api.listTasks).toHaveBeenCalledWith(null);
  });

  it("Save without touching the task leaves setEntryTask uncalled", async () => {
    const { getByText } = render(() => (
      <EditEntryDialog
        entry={entry({ project_id: "p-1", task_id: "t-1" })}
        onClose={vi.fn()}
        onSaved={vi.fn()}
      />
    ));
    fireEvent.click(getByText("Save"));
    await flush();
    expect(api.setEntryTask).not.toHaveBeenCalled();
  });

  it("Delete's inline Cancel resets back to the Delete button without deleting", async () => {
    const onSaved = vi.fn();
    const { getByText, queryByText, getAllByText, findByText } = render(() => (
      <EditEntryDialog
        entry={entry()}
        onClose={vi.fn()}
        onSaved={onSaved}
      />
    ));
    fireEvent.click(getByText("Delete"));
    expect(await findByText("Delete this entry?")).toBeDefined();
    // Two Cancel buttons exist when armed (inline + footer). The first one
    // (inline, sm) resets the destroy state.
    fireEvent.click(getAllByText("Cancel")[0]);
    await flush();
    expect(api.deleteEntry).not.toHaveBeenCalled();
    expect(onSaved).not.toHaveBeenCalled();
    expect(queryByText("Delete this entry?")).toBeNull();
    expect(getByText("Delete")).toBeDefined();
  });
});
