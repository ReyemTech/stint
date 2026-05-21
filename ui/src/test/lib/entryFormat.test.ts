import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import {
  entryDurationSecs,
  entrySyncMeta,
  sumCompletedEntrySeconds,
} from "~/lib/entryFormat";
import type { Entry } from "~/types";

function makeEntry(overrides: Partial<Entry> = {}): Entry {
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

describe("entryDurationSecs", () => {
  it("computes seconds between start and end", () => {
    expect(
      entryDurationSecs("2026-05-20T09:00:00Z", "2026-05-20T09:30:00Z"),
    ).toBe(30 * 60);
  });

  it("uses Date.now() when end is null (running entry)", () => {
    // Pin the clock so the assertion is stable.
    vi.useFakeTimers();
    const now = new Date("2026-05-20T09:45:00Z").getTime();
    vi.setSystemTime(now);
    const got = entryDurationSecs("2026-05-20T09:30:00Z", null);
    expect(got).toBe(15 * 60);
    vi.useRealTimers();
  });

  it("never returns negative when end is before start", () => {
    expect(
      entryDurationSecs("2026-05-20T10:00:00Z", "2026-05-20T09:00:00Z"),
    ).toBe(0);
  });

  it("floors fractional seconds", () => {
    // 59.999s — floors to 59.
    expect(
      entryDurationSecs("2026-05-20T09:00:00.001Z", "2026-05-20T09:01:00.000Z"),
    ).toBe(59);
  });
});

describe("entrySyncMeta", () => {
  it("running takes precedence over sync_state", () => {
    const meta = entrySyncMeta("dirty", true);
    expect(meta.text).toBe("Running");
    expect(meta.tone).toBe("emerald");
    expect(meta.dotTone).toBe("emerald");
  });

  it("synced → Synced (emerald)", () => {
    const meta = entrySyncMeta("synced", false);
    expect(meta).toEqual({ text: "Synced", tone: "emerald", dotTone: "emerald" });
  });

  it("dirty → Edited (amber)", () => {
    const meta = entrySyncMeta("dirty", false);
    expect(meta).toEqual({ text: "Edited", tone: "amber", dotTone: "amber" });
  });

  it("pending_create → Pending (amber)", () => {
    const meta = entrySyncMeta("pending_create", false);
    expect(meta).toEqual({ text: "Pending", tone: "amber", dotTone: "amber" });
  });

  it("pending_delete → Deleting (red)", () => {
    const meta = entrySyncMeta("pending_delete", false);
    expect(meta).toEqual({ text: "Deleting", tone: "red", dotTone: "red" });
  });
});

describe("sumCompletedEntrySeconds", () => {
  beforeEach(() => {
    // Fix the clock — entryDurationSecs in this helper never uses now,
    // but pinning makes the test robust against future drift.
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-05-20T23:59:00Z").getTime());
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("returns 0 for an empty array", () => {
    expect(sumCompletedEntrySeconds([])).toBe(0);
  });

  it("skips running (end_at = null) entries", () => {
    const total = sumCompletedEntrySeconds([
      makeEntry({ end_at: null }),
      makeEntry({
        start_at: "2026-05-20T09:00:00Z",
        end_at: "2026-05-20T10:00:00Z",
      }),
    ]);
    expect(total).toBe(3600);
  });

  it("sums all completed entries", () => {
    const total = sumCompletedEntrySeconds([
      makeEntry({
        start_at: "2026-05-20T09:00:00Z",
        end_at: "2026-05-20T09:30:00Z",
      }),
      makeEntry({
        start_at: "2026-05-20T10:00:00Z",
        end_at: "2026-05-20T10:45:00Z",
      }),
    ]);
    expect(total).toBe(30 * 60 + 45 * 60);
  });

  it("filters to billable entries when onlyBillable is set", () => {
    const total = sumCompletedEntrySeconds(
      [
        makeEntry({
          start_at: "2026-05-20T09:00:00Z",
          end_at: "2026-05-20T10:00:00Z",
          billable: true,
        }),
        makeEntry({
          start_at: "2026-05-20T10:00:00Z",
          end_at: "2026-05-20T11:00:00Z",
          billable: false,
        }),
      ],
      { onlyBillable: true },
    );
    expect(total).toBe(3600);
  });
});
