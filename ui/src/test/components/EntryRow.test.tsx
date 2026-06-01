import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render } from "@solidjs/testing-library";

vi.mock("~/api", () => ({
  api: {
    listProjects: vi.fn().mockResolvedValue([
      { id: "p-1", name: "Tet", color: null, client_id: null, client_name: null, archived: 0 },
      { id: "p-2", name: "Other", color: null, client_id: null, client_name: null, archived: 0 },
    ]),
    listTasks: vi.fn().mockResolvedValue([]),
    updateDescription: vi.fn().mockResolvedValue(undefined),
    setEntryProject: vi.fn().mockResolvedValue(undefined),
    setEntryTask: vi.fn().mockResolvedValue(undefined),
    setEntryBillable: vi.fn().mockResolvedValue(undefined),
    updateEntryTimes: vi.fn().mockResolvedValue(undefined),
    deleteEntry: vi.fn().mockResolvedValue(undefined),
    restartEntry: vi.fn().mockResolvedValue({
      local_uuid: "new-uuid",
      solidtime_id: null,
      description: "deep work",
      project_id: null,
      task_id: null,
      billable: false,
      start_at: new Date().toISOString(),
      end_at: null,
      source: "gui",
    }),
  },
}));

import EntryRow from "~/components/EntryRow";
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

describe("<EntryRow>", () => {
  it("renders description, formatted duration, and Synced pill", () => {
    const { getByText, container } = render(() => <EntryRow entry={entry()} />);
    expect(getByText("deep work")).toBeDefined();
    expect(getByText("00:30:00")).toBeDefined();
    expect(getByText("Synced")).toBeDefined();
    expect(container.querySelector("button")).toBeDefined();
  });

  it("shows (no description) italic placeholder when description is empty", () => {
    const { getByText } = render(() => (
      <EntryRow entry={entry({ description: "" })} />
    ));
    expect(getByText("(no description)")).toBeDefined();
  });

  it("renders the Billable pill only when entry.billable is true", () => {
    const { queryByText } = render(() => (
      <EntryRow entry={entry({ billable: false })} />
    ));
    expect(queryByText("Billable")).toBeNull();
    const { queryByText: q2 } = render(() => (
      <EntryRow entry={entry({ billable: true })} />
    ));
    expect(q2("Billable")).not.toBeNull();
  });

  it("shows the project name pill when projectName prop is set", () => {
    const { getByText } = render(() => (
      <EntryRow entry={entry()} projectName="Tet" />
    ));
    expect(getByText("Tet")).toBeDefined();
  });

  it("shows the task name pill when taskName prop is set", () => {
    const { getByText } = render(() => (
      <EntryRow entry={entry()} taskName="Implement" />
    ));
    expect(getByText("Implement")).toBeDefined();
  });

  it("does not render an empty task pill when taskName is undefined", () => {
    const { queryByText } = render(() => (
      <EntryRow entry={entry()} projectName="Tet" />
    ));
    // Sanity: only Tet + Synced pills are shown.
    expect(queryByText("Implement")).toBeNull();
  });

  it("shows Running pill when end_at is null", () => {
    const { getByText } = render(() => (
      <EntryRow entry={entry({ end_at: null })} />
    ));
    expect(getByText("Running")).toBeDefined();
  });

  it("clicking the row opens the edit dialog", async () => {
    const { container, queryByText } = render(() => <EntryRow entry={entry()} />);
    expect(queryByText("Edit entry")).toBeNull();
    fireEvent.click(container.querySelector("button")!);
    await flush();
    expect(queryByText("Edit entry")).not.toBeNull();
  });

  it("dialog shows the combined project/task picker after the row is clicked", async () => {
    const { container, queryByLabelText } = render(() => (
      <EntryRow entry={entry()} />
    ));
    expect(queryByLabelText("Open project or task list")).toBeNull();
    fireEvent.click(container.querySelector("button")!);
    await flush();
    expect(queryByLabelText("Open project or task list")).not.toBeNull();
  });

  it("renders a Restart button on completed entries", () => {
    const { getByLabelText } = render(() => <EntryRow entry={entry()} />);
    expect(getByLabelText("Restart timer with these details")).toBeDefined();
  });

  it("hides the Restart button on running entries", () => {
    const { queryByLabelText } = render(() => (
      <EntryRow entry={entry({ end_at: null })} />
    ));
    expect(queryByLabelText("Restart timer with these details")).toBeNull();
  });

  it("clicking Restart calls api.restartEntry and fires onChange", async () => {
    const onChange = vi.fn();
    const { getByLabelText } = render(() => (
      <EntryRow entry={entry()} onChange={onChange} />
    ));
    fireEvent.click(getByLabelText("Restart timer with these details"));
    await flush();
    expect(api.restartEntry).toHaveBeenCalledWith("uuid-1");
    expect(onChange).toHaveBeenCalled();
  });

  it("Restart click does NOT open the edit dialog (event isolation)", async () => {
    const { getByLabelText, queryByText } = render(() => (
      <EntryRow entry={entry()} />
    ));
    fireEvent.click(getByLabelText("Restart timer with these details"));
    await flush();
    expect(queryByText("Edit entry")).toBeNull();
  });

  it("Cancel from the dialog closes it without invoking onChange", async () => {
    const onChange = vi.fn();
    const { container, queryByText, getByText } = render(() => (
      <EntryRow entry={entry()} onChange={onChange} />
    ));
    fireEvent.click(container.querySelector("button")!);
    await flush();
    expect(queryByText("Edit entry")).not.toBeNull();
    fireEvent.click(getByText("Cancel"));
    await flush();
    expect(queryByText("Edit entry")).toBeNull();
    expect(onChange).not.toHaveBeenCalled();
  });
});
