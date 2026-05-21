import { describe, expect, it, vi, beforeEach } from "vitest";
import { render } from "@solidjs/testing-library";

vi.mock("~/api", () => ({
  api: {
    listProjects: vi.fn().mockResolvedValue([
      { id: "p-1", name: "Tet", color: null, client_id: null, client_name: null, archived: 0 },
    ]),
  },
}));

import EntryList from "~/components/EntryList";
import { api } from "~/api";
import type { Entry } from "~/types";

const flushMicrotasks = () => new Promise<void>((r) => setTimeout(r, 0));

function entry(overrides: Partial<Entry> = {}): Entry {
  return {
    local_uuid: "uuid",
    solidtime_id: null,
    description: "x",
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
  vi.mocked(api.listProjects).mockClear();
  vi.mocked(api.listProjects).mockResolvedValue([
    { id: "p-1", name: "Tet", color: null, client_id: null, client_name: null, archived: 0 } as never,
  ]);
});

describe("<EntryList>", () => {
  it("renders the empty-state fallback when entries is []", () => {
    const { getByText } = render(() => <EntryList entries={[]} />);
    expect(getByText(/No entries yet today/i)).toBeDefined();
  });

  it("renders one <li> per entry", async () => {
    const entries = [
      entry({ local_uuid: "a", description: "alpha" }),
      entry({ local_uuid: "b", description: "beta" }),
      entry({ local_uuid: "c", description: "gamma" }),
    ];
    const { container } = render(() => <EntryList entries={entries} />);
    await flushMicrotasks();
    expect(container.querySelectorAll("li").length).toBe(3);
  });

  it("looks up project name from the resource and surfaces it to EntryRow", async () => {
    const entries = [entry({ project_id: "p-1" })];
    const { container, findByText } = render(() => (
      <EntryList entries={entries} />
    ));
    await flushMicrotasks();
    // EntryRow renders a Pill with the projectName when it's set.
    const pill = await findByText("Tet");
    expect(pill).toBeDefined();
    expect(container.querySelector("ul")).toBeDefined();
  });
});
