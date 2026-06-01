import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@solidjs/testing-library";

import ProjectTaskPicker from "~/components/ui/ProjectTaskPicker";
import type { Project, Task } from "~/types";

const proj = (over: Partial<Project> = {}): Project => ({
  id: "p-1",
  name: "Tet",
  color: null,
  client_id: null,
  client_name: null,
  archived: 0,
  ...over,
});

const task = (over: Partial<Task> = {}): Task => ({
  solidtime_id: "t-1",
  project_id: "p-1",
  name: "Implement picker",
  done: false,
  ...over,
});

const flush = () => new Promise<void>((r) => setTimeout(r, 0));

/// Kobalte's Popover.Portal renders outside the render container into
/// document.body, so queries inside the dropdown use `screen` (which
/// targets document.body) rather than the render-scoped helpers.
describe("<ProjectTaskPicker>", () => {
  it("renders the trigger with the placeholder when no value is set", () => {
    const { getByLabelText, getByText } = render(() => (
      <ProjectTaskPicker
        value={{ projectId: null, taskId: null }}
        onChange={vi.fn()}
        projects={[proj()]}
        tasks={[task()]}
        placeholder="Choose…"
      />
    ));
    expect(getByLabelText("Open project or task list")).toBeDefined();
    expect(getByText("Choose…")).toBeDefined();
  });

  it("renders the project name when only a project is selected", () => {
    const { getByText } = render(() => (
      <ProjectTaskPicker
        value={{ projectId: "p-1", taskId: null }}
        onChange={vi.fn()}
        projects={[proj({ id: "p-1", name: "Tet" })]}
        tasks={[]}
      />
    ));
    expect(getByText("Tet")).toBeDefined();
  });

  it("renders 'Project / Task' when both are selected", () => {
    const { getByText } = render(() => (
      <ProjectTaskPicker
        value={{ projectId: "p-1", taskId: "t-1" }}
        onChange={vi.fn()}
        projects={[proj({ id: "p-1", name: "Tet" })]}
        tasks={[task({ solidtime_id: "t-1", project_id: "p-1", name: "Refactor" })]}
      />
    ));
    expect(getByText("Tet / Refactor")).toBeDefined();
  });

  it("opens the dropdown and lists 'No project' + the project rows", async () => {
    const { getByLabelText } = render(() => (
      <ProjectTaskPicker
        value={{ projectId: null, taskId: null }}
        onChange={vi.fn()}
        projects={[proj({ name: "Alpha" }), proj({ id: "p-2", name: "Beta" })]}
        tasks={[]}
      />
    ));
    fireEvent.click(getByLabelText("Open project or task list"));
    await flush();
    expect(screen.queryByText("No project")).not.toBeNull();
    expect(screen.queryByText("Alpha")).not.toBeNull();
    expect(screen.queryByText("Beta")).not.toBeNull();
  });

  it("clicking 'No project' fires onChange with both ids null and closes the dropdown", async () => {
    const onChange = vi.fn();
    const { getByLabelText } = render(() => (
      <ProjectTaskPicker
        value={{ projectId: "p-1", taskId: "t-1" }}
        onChange={onChange}
        projects={[proj({ name: "Tet" })]}
        tasks={[task()]}
      />
    ));
    fireEvent.click(getByLabelText("Open project or task list"));
    await flush();
    fireEvent.click(screen.getByText("No project"));
    await flush();
    expect(onChange).toHaveBeenCalledWith({ projectId: null, taskId: null });
    expect(screen.queryByText("No project")).toBeNull();
  });

  it("clicking a project header selects project-only (task_id stays null)", async () => {
    const onChange = vi.fn();
    const { getByLabelText } = render(() => (
      <ProjectTaskPicker
        value={{ projectId: null, taskId: null }}
        onChange={onChange}
        projects={[proj({ id: "p-1", name: "Alpha" })]}
        tasks={[]}
      />
    ));
    fireEvent.click(getByLabelText("Open project or task list"));
    await flush();
    fireEvent.click(screen.getByText("Alpha"));
    await flush();
    expect(onChange).toHaveBeenCalledWith({ projectId: "p-1", taskId: null });
  });

  it("expanding a project reveals its tasks; clicking a task selects project + task", async () => {
    const onChange = vi.fn();
    const { getByLabelText } = render(() => (
      <ProjectTaskPicker
        value={{ projectId: null, taskId: null }}
        onChange={onChange}
        projects={[proj({ id: "p-1", name: "Alpha" })]}
        tasks={[
          task({ solidtime_id: "t-1", project_id: "p-1", name: "First" }),
          task({ solidtime_id: "t-2", project_id: "p-1", name: "Second" }),
        ]}
      />
    ));
    fireEvent.click(getByLabelText("Open project or task list"));
    await flush();
    expect(screen.queryByText("↳ First")).toBeNull();
    fireEvent.click(screen.getByLabelText("Expand"));
    await flush();
    expect(screen.queryByText("↳ First")).not.toBeNull();
    expect(screen.queryByText("↳ Second")).not.toBeNull();
    fireEvent.click(screen.getByText("↳ Second"));
    await flush();
    expect(onChange).toHaveBeenCalledWith({
      projectId: "p-1",
      taskId: "t-2",
    });
  });

  it("auto-expands the project owning the currently selected task on open", async () => {
    const { getByLabelText } = render(() => (
      <ProjectTaskPicker
        value={{ projectId: "p-1", taskId: "t-1" }}
        onChange={vi.fn()}
        projects={[proj({ id: "p-1", name: "Alpha" })]}
        tasks={[task({ solidtime_id: "t-1", project_id: "p-1", name: "First" })]}
      />
    ));
    fireEvent.click(getByLabelText("Open project or task list"));
    await flush();
    expect(screen.queryByText("↳ First")).not.toBeNull();
  });

  it("smart search auto-expands projects with matching tasks", async () => {
    const { getByLabelText } = render(() => (
      <ProjectTaskPicker
        value={{ projectId: null, taskId: null }}
        onChange={vi.fn()}
        projects={[
          proj({ id: "p-1", name: "Alpha" }),
          proj({ id: "p-2", name: "Beta" }),
        ]}
        tasks={[
          task({ solidtime_id: "t-1", project_id: "p-1", name: "Refactor picker" }),
          task({ solidtime_id: "t-2", project_id: "p-2", name: "Unrelated" }),
        ]}
      />
    ));
    fireEvent.click(getByLabelText("Open project or task list"));
    await flush();
    const search = screen.getByPlaceholderText(
      "Search projects + tasks…",
    ) as HTMLInputElement;
    search.value = "refactor";
    fireEvent.input(search);
    await flush();
    expect(screen.queryByText("↳ Refactor picker")).not.toBeNull();
    expect(screen.queryByText("Beta")).toBeNull();
  });

  it("filters out tasks marked done", async () => {
    const { getByLabelText } = render(() => (
      <ProjectTaskPicker
        value={{ projectId: "p-1", taskId: null }}
        onChange={vi.fn()}
        projects={[proj({ id: "p-1", name: "Alpha" })]}
        tasks={[
          task({ solidtime_id: "t-1", project_id: "p-1", name: "Active", done: false }),
          task({ solidtime_id: "t-2", project_id: "p-1", name: "Finished", done: true }),
        ]}
      />
    ));
    fireEvent.click(getByLabelText("Open project or task list"));
    await flush();
    // Project auto-expands because the parent has projectId in value.
    // Expand explicitly in case auto-expand only triggers when taskId is set.
    fireEvent.click(screen.getByLabelText("Expand"));
    await flush();
    expect(screen.queryByText("↳ Active")).not.toBeNull();
    expect(screen.queryByText("↳ Finished")).toBeNull();
  });

  it("shows an empty-state message when search has no matches", async () => {
    const { getByLabelText } = render(() => (
      <ProjectTaskPicker
        value={{ projectId: null, taskId: null }}
        onChange={vi.fn()}
        projects={[proj({ name: "Alpha" })]}
        tasks={[]}
      />
    ));
    fireEvent.click(getByLabelText("Open project or task list"));
    await flush();
    const search = screen.getByPlaceholderText(
      "Search projects + tasks…",
    ) as HTMLInputElement;
    search.value = "nonexistent";
    fireEvent.input(search);
    await flush();
    expect(screen.queryByText(/No projects or tasks match/)).not.toBeNull();
  });
});
