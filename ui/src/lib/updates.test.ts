import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { checkForUpdates, getChannel, setChannel } from "./updates";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("updates store", () => {
  beforeEach(() => vi.clearAllMocks());

  it("checkForUpdates returns the IPC payload", async () => {
    (invoke as any).mockResolvedValueOnce("stable");          // getChannel → settings_get
    (invoke as any).mockResolvedValueOnce({
      available: true,
      current_version: "0.1.0",
      latest_version: "0.2.0",
      notes: "fixes",
    });
    const info = await checkForUpdates();
    expect(info.available).toBe(true);
    expect(info.latest_version).toBe("0.2.0");
  });

  it("getChannel defaults to stable", async () => {
    (invoke as any).mockResolvedValueOnce(null);
    expect(await getChannel()).toBe("stable");
  });

  it("setChannel forwards to settings", async () => {
    (invoke as any).mockResolvedValueOnce(undefined);
    await setChannel("beta");
    expect(invoke).toHaveBeenCalledWith("settings_set", { key: "update.channel", value: "beta" });
  });
});
