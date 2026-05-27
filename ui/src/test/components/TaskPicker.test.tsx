import { describe, expect, it, vi } from "vitest";
import { render } from "@solidjs/testing-library";
import TaskPicker from "~/components/ui/TaskPicker";
import type { Task } from "~/types";

function fixture(): Task[] {
  return [
    {
      solidtime_id: "t-1",
      project_id: "p-1",
      name: "Implement",
      done: false,
    },
    {
      solidtime_id: "t-2",
      project_id: "p-1",
      name: "Review",
      done: false,
    },
  ];
}

describe("<TaskPicker>", () => {
  it("renders the input with placeholder when a project is selected", () => {
    const { container } = render(() => (
      <TaskPicker
        value={null}
        onChange={vi.fn()}
        tasks={fixture()}
        projectSelected={true}
        placeholder="Pick a task"
      />
    ));
    const input = container.querySelector("input") as HTMLInputElement;
    expect(input).toBeTruthy();
    expect(input.placeholder).toBe("Pick a task");
    expect(input.disabled).toBe(false);
  });

  it("disables the input when no project is selected", () => {
    const { container } = render(() => (
      <TaskPicker
        value={null}
        onChange={vi.fn()}
        tasks={[]}
        projectSelected={false}
      />
    ));
    const input = container.querySelector("input") as HTMLInputElement;
    expect(input.disabled).toBe(true);
  });

  it("renders an open-list trigger labelled for assistive tech", () => {
    const { getByLabelText } = render(() => (
      <TaskPicker
        value={null}
        onChange={vi.fn()}
        tasks={fixture()}
        projectSelected={true}
      />
    ));
    expect(getByLabelText("Open task list")).toBeTruthy();
  });

  it("defaults the placeholder when none is provided", () => {
    const { container } = render(() => (
      <TaskPicker
        value={null}
        onChange={vi.fn()}
        tasks={fixture()}
        projectSelected={true}
      />
    ));
    const input = container.querySelector("input") as HTMLInputElement;
    expect(input.placeholder).toBe("Select task…");
  });

  it("leaves the input empty when value is null so the placeholder shows", () => {
    // Same regression check as ProjectPicker: the synthetic NO_TASK row
    // (id="") must not auto-select when value is null.
    const { container } = render(() => (
      <TaskPicker
        value={null}
        onChange={vi.fn()}
        tasks={fixture()}
        projectSelected={true}
        placeholder="Pick a task"
      />
    ));
    const input = container.querySelector("input") as HTMLInputElement;
    expect(input.value).toBe("");
  });
});
