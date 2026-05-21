import { describe, expect, it } from "vitest";
import {
  isoMinutesAgo,
  parseLocalHHMMTodayOrYesterday,
  startAtLabel,
} from "~/lib/startAt";

describe("isoMinutesAgo", () => {
  it("returns the supplied now minus n minutes as ISO", () => {
    const now = new Date("2026-05-20T12:00:00Z");
    expect(isoMinutesAgo(15, now)).toBe("2026-05-20T11:45:00.000Z");
  });
});

describe("parseLocalHHMMTodayOrYesterday", () => {
  it("rejects invalid input", () => {
    expect(parseLocalHHMMTodayOrYesterday("nope")).toBeNull();
    expect(parseLocalHHMMTodayOrYesterday("25:00")).toBeNull();
    expect(parseLocalHHMMTodayOrYesterday("12:99")).toBeNull();
    expect(parseLocalHHMMTodayOrYesterday("")).toBeNull();
  });

  it("anchors a same-day HH:MM in the local-today date", () => {
    const now = new Date("2026-05-20T18:00:00");
    const out = parseLocalHHMMTodayOrYesterday("09:30", now);
    expect(out).not.toBeNull();
    const parsed = new Date(out!);
    expect(parsed.getDate()).toBe(20);
    expect(parsed.getHours()).toBe(9);
    expect(parsed.getMinutes()).toBe(30);
  });

  it("shifts to yesterday when HH:MM is after now", () => {
    // Local-time now is 06:00. Requesting 09:30 (still future today) should
    // anchor to yesterday at 09:30.
    const now = new Date("2026-05-20T06:00:00");
    const out = parseLocalHHMMTodayOrYesterday("09:30", now);
    expect(out).not.toBeNull();
    const parsed = new Date(out!);
    expect(parsed.getDate()).toBe(19);
    expect(parsed.getHours()).toBe(9);
    expect(parsed.getMinutes()).toBe(30);
  });
});

describe("startAtLabel", () => {
  it("returns 'Start now' when value is null", () => {
    expect(startAtLabel(null)).toBe("Start now");
  });

  it("returns 'Start now' when value is within the last minute", () => {
    const now = new Date("2026-05-20T12:00:00Z");
    expect(startAtLabel("2026-05-20T11:59:30Z", now)).toBe("Start now");
  });

  it("formats minute offsets up to 60", () => {
    const now = new Date("2026-05-20T12:00:00Z");
    expect(startAtLabel("2026-05-20T11:45:00Z", now)).toBe("Start 15 min ago");
  });

  it("formats hour offsets above 60 min", () => {
    const now = new Date("2026-05-20T12:00:00Z");
    expect(startAtLabel("2026-05-20T10:30:00Z", now)).toBe("Start 1.5h ago");
  });
});
