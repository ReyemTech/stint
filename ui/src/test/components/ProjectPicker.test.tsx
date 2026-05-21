import { describe, expect, it, vi } from "vitest";
import { render } from "@solidjs/testing-library";
import ProjectPicker from "~/components/ui/ProjectPicker";
import type { Project } from "~/types";

function fixture(): Project[] {
  return [
    {
      id: "p-1",
      name: "Site",
      color: null,
      client_id: "c-1",
      client_name: "Acme",
      archived: 0,
    },
    {
      id: "p-2",
      name: "Internal",
      color: null,
      client_id: null,
      client_name: null,
      archived: 0,
    },
  ];
}

describe("<ProjectPicker>", () => {
  it("renders an input with the placeholder", () => {
    const { container } = render(() => (
      <ProjectPicker
        value={null}
        onChange={vi.fn()}
        projects={fixture()}
        placeholder="Pick a project"
      />
    ));
    const input = container.querySelector("input") as HTMLInputElement;
    expect(input).toBeTruthy();
    expect(input.placeholder).toBe("Pick a project");
  });

  it("renders an open-list trigger labelled for assistive tech", () => {
    const { getByLabelText } = render(() => (
      <ProjectPicker value={null} onChange={vi.fn()} projects={fixture()} />
    ));
    expect(getByLabelText("Open project list")).toBeTruthy();
  });

  it("defaults the placeholder when none is provided", () => {
    const { container } = render(() => (
      <ProjectPicker value={null} onChange={vi.fn()} projects={fixture()} />
    ));
    const input = container.querySelector("input") as HTMLInputElement;
    expect(input.placeholder).toBe("Select project…");
  });

  it("applies the small size class when size=sm", () => {
    const { container } = render(() => (
      <ProjectPicker
        value={null}
        onChange={vi.fn()}
        projects={fixture()}
        size="sm"
      />
    ));
    const control = container.querySelector("[class*='text-[12px]']");
    expect(control).toBeTruthy();
  });
});
