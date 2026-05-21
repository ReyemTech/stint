import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render } from "@solidjs/testing-library";
import type { SyncError } from "~/types";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("~/api", () => ({
  api: {
    listSyncErrors: vi.fn().mockResolvedValue([]),
    deleteEntry: vi.fn().mockResolvedValue(undefined),
    listProjects: vi.fn().mockResolvedValue([]),
    updateDescription: vi.fn().mockResolvedValue(undefined),
    setEntryProject: vi.fn().mockResolvedValue(undefined),
    setEntryBillable: vi.fn().mockResolvedValue(undefined),
    updateEntryTimes: vi.fn().mockResolvedValue(undefined),
  },
}));

import SyncErrorBanner from "~/components/SyncErrorBanner";
import { api } from "~/api";

const flush = () => new Promise<void>((r) => setTimeout(r, 0));

function err(overrides: Partial<SyncError> = {}): SyncError {
  return {
    queue_id: 1,
    local_uuid: "uuid-1",
    op: "create_entry",
    attempts: 5,
    last_error:
      'solidtime API error: status 400, body: {"error":true,"key":"overlapping_time_entry"}',
    next_try_at: "2027-05-21T00:00:00Z",
    abandoned: true,
    description: "Liberty Issue",
    start_at: "2026-05-21T13:39:14Z",
    end_at: "2026-05-21T16:21:44Z",
    ...overrides,
  };
}

beforeEach(() => {
  vi.mocked(api.listSyncErrors).mockReset();
  vi.mocked(api.listSyncErrors).mockResolvedValue([]);
  vi.mocked(api.deleteEntry).mockClear();
});

describe("<SyncErrorBanner>", () => {
  it("renders nothing when there are no errors", async () => {
    const { container } = render(() => <SyncErrorBanner />);
    await flush();
    expect(container.querySelector('[role="alert"]')).toBeNull();
  });

  it("renders nothing when failures exist but none are abandoned (still retrying)", async () => {
    vi.mocked(api.listSyncErrors).mockResolvedValue([
      err({ abandoned: false, last_error: "transient 500" }),
    ]);
    const { container } = render(() => <SyncErrorBanner />);
    await flush();
    expect(container.querySelector('[role="alert"]')).toBeNull();
  });

  it("renders an alert with the abandoned count", async () => {
    vi.mocked(api.listSyncErrors).mockResolvedValue([err()]);
    const { findByRole, findByText } = render(() => <SyncErrorBanner />);
    expect(await findByRole("alert")).toBeDefined();
    expect(await findByText(/1 entry couldn't sync/)).toBeDefined();
  });

  it("expands to show the description + a friendly explanation of overlap errors", async () => {
    vi.mocked(api.listSyncErrors).mockResolvedValue([err()]);
    const { findByText, getByText } = render(() => <SyncErrorBanner />);
    fireEvent.click(await findByText(/1 entry couldn't sync/));
    await flush();
    expect(getByText("Liberty Issue")).toBeDefined();
    expect(
      getByText(/Conflicts with another entry in Solidtime/),
    ).toBeDefined();
  });

  it("Delete entry calls api.deleteEntry with the local_uuid", async () => {
    vi.mocked(api.listSyncErrors).mockResolvedValue([err()]);
    const { findByText, getByText } = render(() => <SyncErrorBanner />);
    fireEvent.click(await findByText(/1 entry couldn't sync/));
    await flush();
    fireEvent.click(getByText("Delete entry"));
    await flush();
    expect(api.deleteEntry).toHaveBeenCalledWith("uuid-1");
  });
});
