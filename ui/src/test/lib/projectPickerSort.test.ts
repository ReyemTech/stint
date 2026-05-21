import { describe, expect, it } from "vitest";
import { buildPickerOptions } from "~/lib/projectPickerSort";
import type { Project } from "~/types";

function p(overrides: Partial<Project>): Project {
  return {
    id: "p",
    name: "Project",
    color: null,
    client_id: null,
    client_name: null,
    archived: 0,
    ...overrides,
  };
}

describe("buildPickerOptions", () => {
  it("returns an empty list when no projects are given", () => {
    expect(buildPickerOptions([])).toEqual([]);
  });

  it("drops archived projects", () => {
    const out = buildPickerOptions([
      p({ id: "live", name: "Live", archived: 0 }),
      p({ id: "old", name: "Old", archived: 1 }),
    ]);
    expect(out.map((o) => o.id)).toEqual(["live"]);
  });

  it("places projects without a client at the bottom", () => {
    const out = buildPickerOptions([
      p({ id: "no1", name: "Alpha" }),
      p({ id: "yes1", name: "Beta", client_id: "c1", client_name: "Acme" }),
      p({ id: "no2", name: "Charlie" }),
    ]);
    expect(out.map((o) => o.id)).toEqual(["yes1", "no1", "no2"]);
  });

  it("sorts clientful projects by client name then project name", () => {
    const out = buildPickerOptions([
      p({ id: "z", name: "Zebra", client_id: "ac", client_name: "Acme" }),
      p({ id: "y", name: "Yak",   client_id: "ac", client_name: "Acme" }),
      p({ id: "x", name: "Xeno",  client_id: "bg", client_name: "Beta" }),
    ]);
    expect(out.map((o) => o.id)).toEqual(["y", "z", "x"]);
  });

  it("sorts clientless projects by project name", () => {
    const out = buildPickerOptions([
      p({ id: "c", name: "Charlie" }),
      p({ id: "a", name: "Alpha" }),
      p({ id: "b", name: "Bravo" }),
    ]);
    expect(out.map((o) => o.id)).toEqual(["a", "b", "c"]);
  });

  it("includes clientName on each option (used by the per-item subtitle)", () => {
    const [first] = buildPickerOptions([
      p({ id: "x", name: "X", client_id: "c", client_name: "Acme" }),
    ]);
    expect(first.clientName).toBe("Acme");
  });
});
