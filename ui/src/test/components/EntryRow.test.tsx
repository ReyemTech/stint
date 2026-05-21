import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render } from "@solidjs/testing-library";

vi.mock("~/api", () => ({
  api: {
    listProjects: vi.fn().mockResolvedValue([
      { id: "p-1", name: "Tet", color: null, client_id: null, client_name: null, archived: 0 },
      { id: "p-2", name: "Other", color: null, client_id: null, client_name: null, archived: 0 },
    ]),
    updateDescription: vi.fn().mockResolvedValue(undefined),
    setEntryProject: vi.fn().mockResolvedValue(undefined),
    setEntryBillable: vi.fn().mockResolvedValue(undefined),
  },
}));

import EntryRow from "~/components/EntryRow";
import { api } from "~/api";
import type { Entry } from "~/types";

const flushMicrotasks = () => new Promise<void>((r) => setTimeout(r, 0));

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
    const { queryByText, rerender } = render(() => (
      <EntryRow entry={entry({ billable: false })} />
    )) as any;
    expect(queryByText("Billable")).toBeNull();
    // separate render for true case (rerender isn't part of the solid lib)
    const { queryByText: q2 } = render(() => (
      <EntryRow entry={entry({ billable: true })} />
    ));
    expect(q2("Billable")).not.toBeNull();
    void rerender;
  });

  it("shows the project name pill when projectName prop is set", () => {
    const { getByText } = render(() => (
      <EntryRow entry={entry()} projectName="Tet" />
    ));
    expect(getByText("Tet")).toBeDefined();
  });

  it("shows Running pill when end_at is null", () => {
    const { getByText } = render(() => (
      <EntryRow entry={entry({ end_at: null })} />
    ));
    expect(getByText("Running")).toBeDefined();
  });

  it("clicking the row toggles the editor panel open", async () => {
    const { container, queryByText } = render(() => <EntryRow entry={entry()} />);
    expect(queryByText("Description")).toBeNull();
    fireEvent.click(container.querySelector("button")!);
    await flushMicrotasks();
    expect(queryByText("Description")).not.toBeNull();
    expect(container.querySelector('input[type="text"]')).not.toBeNull();
  });

  it("opens with the ProjectPicker visible inside the editor panel", async () => {
    const { container, queryByLabelText } = render(() => (
      <EntryRow entry={entry()} />
    ));
    expect(queryByLabelText("Open project list")).toBeNull();
    fireEvent.click(container.querySelector("button")!);
    await flushMicrotasks();
    expect(queryByLabelText("Open project list")).not.toBeNull();
  });

  it("toggling the Billable Toggle calls api.setEntryBillable", async () => {
    const { container, getByRole } = render(() => (
      <EntryRow entry={entry({ billable: false })} />
    ));
    fireEvent.click(container.querySelector("button")!);
    await flushMicrotasks();
    fireEvent.click(getByRole("switch"));
    await flushMicrotasks();
    expect(api.setEntryBillable).toHaveBeenCalledWith("uuid-1", true);
  });

  it("editing the description and blurring calls api.updateDescription", async () => {
    const { container } = render(() => <EntryRow entry={entry()} />);
    fireEvent.click(container.querySelector("button")!);
    await flushMicrotasks();
    const input = container.querySelector('input[type="text"]') as HTMLInputElement;
    input.value = "renamed";
    fireEvent.input(input);
    fireEvent.blur(input);
    await flushMicrotasks();
    expect(api.updateDescription).toHaveBeenCalledWith("uuid-1", "renamed");
  });

  it("Delete button invokes onDelete with the local uuid", async () => {
    const onDelete = vi.fn();
    const { container, findByRole } = render(() => (
      <EntryRow entry={entry()} onDelete={onDelete} />
    ));
    fireEvent.click(container.querySelector("button")!);
    await flushMicrotasks();
    const del = await findByRole("button", { name: /Delete entry/i });
    fireEvent.click(del);
    expect(onDelete).toHaveBeenCalledWith("uuid-1");
  });
});
