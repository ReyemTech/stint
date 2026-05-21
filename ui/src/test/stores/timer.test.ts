import { describe, expect, it, vi, beforeEach } from "vitest";
import { createRoot } from "solid-js";

// Mock the IPC layer before importing the store. The store reads from
// `~/api` and `@tauri-apps/api/event` at module-eval time — must mock
// before the first import statement that pulls those in.
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("~/api", () => ({
  api: {
    getRunningTimer: vi.fn(),
    startTimer: vi.fn().mockResolvedValue("local-uuid-1"),
    stopTimer: vi.fn().mockResolvedValue(""),
  },
}));

import { useTimerStore } from "~/stores/timer";
import { api } from "~/api";

// Helper: wait for the initial async refresh inside the store to settle.
const flushMicrotasks = () => new Promise<void>((r) => setTimeout(r, 0));

describe("useTimerStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.getRunningTimer).mockResolvedValue(null);
  });

  it("calls getRunningTimer on construction", async () => {
    await createRoot(async (dispose) => {
      useTimerStore();
      await flushMicrotasks();
      expect(api.getRunningTimer).toHaveBeenCalled();
      dispose();
    });
  });

  it("running is null when no timer is active on the backend", async () => {
    await createRoot(async (dispose) => {
      const store = useTimerStore();
      await flushMicrotasks();
      expect(store.running()).toBeNull();
      expect(store.elapsedSecs()).toBe(0);
      dispose();
    });
  });

  it("populates running and elapsedSecs from backend state", async () => {
    const startedAt = new Date(Date.now() - 10_000).toISOString();
    vi.mocked(api.getRunningTimer).mockResolvedValue({
      local_uuid: "uuid-1",
      description: "deep work",
      start_at: startedAt,
      project_id: null,
      billable: false,
    });

    await createRoot(async (dispose) => {
      const store = useTimerStore();
      await flushMicrotasks();
      expect(store.running()?.local_uuid).toBe("uuid-1");
      expect(store.running()?.description).toBe("deep work");
      // ~10 seconds elapsed (allow ±2s for test latency).
      expect(store.elapsedSecs()).toBeGreaterThanOrEqual(9);
      expect(store.elapsedSecs()).toBeLessThanOrEqual(12);
      dispose();
    });
  });

  it("start() forwards args to api.startTimer and refreshes", async () => {
    await createRoot(async (dispose) => {
      const store = useTimerStore();
      await flushMicrotasks();
      vi.mocked(api.getRunningTimer).mockClear();

      await store.start("write tests", "p-1", true);
      expect(api.startTimer).toHaveBeenCalledWith(
        "write tests",
        "p-1",
        null,
        true,
        null,
      );
      // refresh fires after start.
      expect(api.getRunningTimer).toHaveBeenCalled();
      dispose();
    });
  });

  it("start() with no project passes undefined → null to api", async () => {
    await createRoot(async (dispose) => {
      const store = useTimerStore();
      await flushMicrotasks();
      await store.start("solo");
      expect(api.startTimer).toHaveBeenCalledWith(
        "solo",
        null,
        null,
        false,
        null,
      );
      dispose();
    });
  });

  it("stop() invokes api.stopTimer and refreshes", async () => {
    await createRoot(async (dispose) => {
      const store = useTimerStore();
      await flushMicrotasks();
      vi.mocked(api.getRunningTimer).mockClear();

      await store.stop();
      expect(api.stopTimer).toHaveBeenCalled();
      expect(api.getRunningTimer).toHaveBeenCalled();
      dispose();
    });
  });

  it("refresh() swallows backend errors and logs to console.warn", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    vi.mocked(api.getRunningTimer).mockRejectedValueOnce(new Error("backend down"));

    await createRoot(async (dispose) => {
      const store = useTimerStore();
      await flushMicrotasks();
      // running remains null; no exception escapes.
      expect(store.running()).toBeNull();
      expect(warn).toHaveBeenCalled();
      dispose();
    });
    warn.mockRestore();
  });
});
