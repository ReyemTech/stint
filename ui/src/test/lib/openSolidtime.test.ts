import { describe, expect, it, vi, beforeEach } from "vitest";

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn().mockResolvedValue(undefined),
}));
vi.mock("~/api", () => ({
  api: {
    solidtimeUrl: vi.fn(),
  },
}));

import { openSolidtime } from "~/lib/openSolidtime";
import { api } from "~/api";
import { openUrl } from "@tauri-apps/plugin-opener";

describe("openSolidtime", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("is a no-op when the configured URL is null", async () => {
    vi.mocked(api.solidtimeUrl).mockResolvedValue(null);
    await openSolidtime();
    expect(openUrl).not.toHaveBeenCalled();
  });

  it("opens the configured URL when no path is given", async () => {
    vi.mocked(api.solidtimeUrl).mockResolvedValue("https://example.com");
    await openSolidtime();
    expect(openUrl).toHaveBeenCalledWith("https://example.com");
  });

  it("joins with a leading slash on the path", async () => {
    vi.mocked(api.solidtimeUrl).mockResolvedValue("https://example.com");
    await openSolidtime("/foo");
    expect(openUrl).toHaveBeenCalledWith("https://example.com/foo");
  });

  it("normalises a path without a leading slash", async () => {
    vi.mocked(api.solidtimeUrl).mockResolvedValue("https://example.com");
    await openSolidtime("bar");
    expect(openUrl).toHaveBeenCalledWith("https://example.com/bar");
  });
});
