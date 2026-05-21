import { describe, expect, it } from "vitest";
import { render } from "@solidjs/testing-library";
import Duration, { formatDuration } from "~/components/Duration";

describe("formatDuration", () => {
  it("formats zero as 00:00:00", () => {
    expect(formatDuration(0)).toBe("00:00:00");
  });

  it("formats sub-minute durations", () => {
    expect(formatDuration(5)).toBe("00:00:05");
    expect(formatDuration(59)).toBe("00:00:59");
  });

  it("rolls into minutes", () => {
    expect(formatDuration(60)).toBe("00:01:00");
    expect(formatDuration(125)).toBe("00:02:05");
  });

  it("rolls into hours", () => {
    expect(formatDuration(3600)).toBe("01:00:00");
    expect(formatDuration(3661)).toBe("01:01:01");
  });

  it("supports hours above 9", () => {
    expect(formatDuration(36 * 3600 + 12 * 60 + 7)).toBe("36:12:07");
  });
});

describe("<Duration>", () => {
  it("renders the formatted seconds inside a tabular-numeric span", () => {
    const { container } = render(() => <Duration seconds={3661} />);
    const span = container.querySelector("span")!;
    expect(span.textContent).toBe("01:01:01");
    expect(span.className).toContain("tabular-nums");
  });
});
