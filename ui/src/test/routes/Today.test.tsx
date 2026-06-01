import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render } from "@solidjs/testing-library";
import { createSignal } from "solid-js";
import type { Entry, RunningTimer } from "~/types";

const [running, setRunning] = createSignal<RunningTimer | null>(null);
const [elapsedSecs] = createSignal(0);

const storeMock = {
  running,
  elapsedSecs,
  refresh: vi.fn(),
  start: vi.fn().mockResolvedValue(undefined),
  stop: vi.fn().mockResolvedValue(undefined),
};
vi.mock("~/stores/timer", () => ({
  useTimerStore: () => storeMock,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("~/api", () => ({
  api: {
    listToday: vi.fn().mockResolvedValue([]),
    listProjects: vi.fn().mockResolvedValue([]),
    listTasks: vi.fn().mockResolvedValue([]),
    syncNow: vi.fn().mockResolvedValue(0),
    deleteEntry: vi.fn().mockResolvedValue(undefined),
    listSyncErrors: vi.fn().mockResolvedValue([]),
  },
  calendarApi: {
    listAccounts: vi.fn().mockResolvedValue([]),
    listEventsInRange: vi.fn().mockResolvedValue([]),
  },
  pullNow: vi.fn().mockResolvedValue({
    adopted: null,
    conflict: null,
    inserted: 0,
    updated: 0,
    deleted: 0,
  }),
  conflictResolve: vi.fn(),
}));

vi.mock("~/lib/openSolidtime", () => ({
  openSolidtime: vi.fn().mockResolvedValue(undefined),
}));

import Today from "~/routes/Today";
import { api, pullNow } from "~/api";
import { MemoryRouter, Route } from "@solidjs/router";

/// Today calls `useSearchParams` for the `?entry=<uuid>` deep-link
/// highlight path. `useSearchParams` aborts when used outside a Router,
/// so each test renders Today through a MemoryRouter shim. Using
/// MemoryRouter rather than the real Router keeps jsdom happy (no
/// `window.history` manipulation surprises across tests).
const renderToday = () =>
  render(() => (
    <MemoryRouter>
      <Route path="*" component={Today} />
    </MemoryRouter>
  ));

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
  setRunning(null);
  vi.mocked(api.listToday).mockResolvedValue([]);
  vi.mocked(api.listProjects).mockResolvedValue([]);
  vi.mocked(api.syncNow).mockResolvedValue(0);
  vi.mocked(pullNow).mockResolvedValue({
    adopted: null,
    conflict: null,
    inserted: 0,
    updated: 0,
    deleted: 0,
  });
  vi.mocked(api.deleteEntry).mockClear();
});

describe("<Today>", () => {
  it("renders the Today heading + the 'no entries' empty state", async () => {
    const { getByRole, findByText } = renderToday();
    await flushMicrotasks();
    expect(getByRole("heading", { name: "Today", level: 1 })).toBeDefined();
    expect(await findByText(/No entries yet today/)).toBeDefined();
  });

  it("shows 'Synced' badge when there are no pending entries", async () => {
    vi.mocked(api.listToday).mockResolvedValue([entry({ sync_state: "synced" })]);
    const { findByText } = renderToday();
    expect(await findByText("Synced")).toBeDefined();
  });

  it("shows the pending count when entries have unsynced state", async () => {
    vi.mocked(api.listToday).mockResolvedValue([
      entry({ local_uuid: "a", sync_state: "pending_create" }),
      entry({ local_uuid: "b", sync_state: "dirty" }),
    ]);
    const { findByText } = renderToday();
    expect(await findByText("Sync (2)")).toBeDefined();
  });

  it("clicking the sync badge runs pullNow + api.syncNow and surfaces a message", async () => {
    vi.mocked(api.syncNow).mockResolvedValue(3);
    const { findByText, getByTitle } = renderToday();
    await flushMicrotasks();
    fireEvent.click(getByTitle(/Sync with Solidtime/));
    await flushMicrotasks();
    await flushMicrotasks();
    expect(pullNow).toHaveBeenCalled();
    expect(api.syncNow).toHaveBeenCalled();
    expect(await findByText(/Synced 3 items/)).toBeDefined();
  });

  it("shows a Sync failed message when syncNow throws", async () => {
    vi.mocked(api.syncNow).mockRejectedValue(new Error("network down"));
    const { findByText, getByTitle } = renderToday();
    await flushMicrotasks();
    fireEvent.click(getByTitle(/Sync with Solidtime/));
    await flushMicrotasks();
    expect(await findByText(/Sync failed: network down/)).toBeDefined();
  });

  it("entries render via EntryList when present", async () => {
    vi.mocked(api.listToday).mockResolvedValue([
      entry({ description: "alpha task", sync_state: "synced" }),
    ]);
    const { findByText } = renderToday();
    expect(await findByText("alpha task")).toBeDefined();
  });
});
