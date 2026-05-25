import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render } from "@solidjs/testing-library";

// Hoisted helpers shared between mock factories and tests.
const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn().mockResolvedValue(undefined),
}));

import UpdatesPanel from "~/routes/Settings/UpdatesPanel";
import { openUrl } from "@tauri-apps/plugin-opener";
import { requestCheckForUpdates } from "~/lib/updates";

const flushMicrotasks = () => new Promise<void>((r) => setTimeout(r, 0));
async function flushAll() {
  for (let i = 0; i < 5; i++) await flushMicrotasks();
}

type UpdateInfo = {
  available: boolean;
  current_version: string;
  latest_version: string | null;
  notes: string | null;
};

const upToDate: UpdateInfo = {
  available: false,
  current_version: "0.2.0",
  latest_version: "0.2.0",
  notes: null,
};

const updateAvailable: UpdateInfo = {
  available: true,
  current_version: "0.2.0",
  latest_version: "0.3.0",
  notes: "Lots of fixes\n- one\n- two",
};

const updateAvailableChangelog: UpdateInfo = {
  available: true,
  current_version: "0.2.0",
  latest_version: "0.3.0",
  notes: "see CHANGELOG.md",
};

// Build an invoke implementation that responds based on command name.
function makeInvokeImpl(handlers: Record<string, () => unknown | Promise<unknown>>) {
  return (cmd: string) => {
    const handler = handlers[cmd];
    if (!handler) return Promise.reject(new Error(`unexpected invoke: ${cmd}`));
    try {
      const out = handler();
      return out instanceof Promise ? out : Promise.resolve(out);
    } catch (e) {
      return Promise.reject(e);
    }
  };
}

beforeEach(() => {
  invokeMock.mockReset();
  vi.mocked(openUrl).mockClear();
});

describe("<UpdatesPanel>", () => {
  it("renders the channel select and Check now button on first mount", async () => {
    invokeMock.mockImplementation(
      makeInvokeImpl({
        settings_get: () => "stable",
      }),
    );
    const { findByRole, getByRole, getByText } = render(() => <UpdatesPanel />);
    await flushAll();
    expect(await findByRole("button", { name: "Check now" })).toBeDefined();
    expect(getByRole("combobox")).toBeDefined();
    // No version known yet so the "Current version" line shows the em dash.
    expect(getByText(/Current version: —/)).toBeDefined();
  });

  it("Check now displays the up-to-date message when no update is available", async () => {
    invokeMock.mockImplementation(
      makeInvokeImpl({
        settings_get: () => "stable",
        check_for_updates: () => upToDate,
      }),
    );
    const { findByRole, findByText } = render(() => <UpdatesPanel />);
    await flushAll();
    fireEvent.click(await findByRole("button", { name: "Check now" }));
    await flushAll();
    expect(invokeMock).toHaveBeenCalledWith("check_for_updates", { channel: "stable" });
    expect(await findByText(/You're on the latest version \(0\.2\.0\)/)).toBeDefined();
  });

  it("Check now displays the available card + plain-text notes when an update is found", async () => {
    invokeMock.mockImplementation(
      makeInvokeImpl({
        settings_get: () => "stable",
        check_for_updates: () => updateAvailable,
      }),
    );
    const { findByRole, findByText, getByText } = render(() => <UpdatesPanel />);
    await flushAll();
    fireEvent.click(await findByRole("button", { name: "Check now" }));
    await flushAll();
    expect(await findByText(/Update available: 0\.3\.0/)).toBeDefined();
    expect(getByText(/Lots of fixes/)).toBeDefined();
    expect(await findByRole("button", { name: "Install" })).toBeDefined();
  });

  it("renders a CHANGELOG link when notes is the placeholder", async () => {
    invokeMock.mockImplementation(
      makeInvokeImpl({
        settings_get: () => "stable",
        check_for_updates: () => updateAvailableChangelog,
      }),
    );
    const { findByRole } = render(() => <UpdatesPanel />);
    await flushAll();
    fireEvent.click(await findByRole("button", { name: "Check now" }));
    await flushAll();
    const link = await findByRole("button", { name: "CHANGELOG.md" });
    fireEvent.click(link);
    await flushAll();
    expect(openUrl).toHaveBeenCalledWith(
      "https://github.com/reyemtech/stint/blob/v0.3.0/CHANGELOG.md",
    );
  });

  it("Check now surfaces an error message when the IPC rejects", async () => {
    invokeMock.mockImplementation(
      makeInvokeImpl({
        settings_get: () => "stable",
        check_for_updates: () => {
          throw new Error("offline");
        },
      }),
    );
    const { findByRole, findByText } = render(() => <UpdatesPanel />);
    await flushAll();
    fireEvent.click(await findByRole("button", { name: "Check now" }));
    await flushAll();
    expect(await findByText("offline")).toBeDefined();
  });

  it("Install flips the card to 'Update installed' + Restart Stint button", async () => {
    invokeMock.mockImplementation(
      makeInvokeImpl({
        settings_get: () => "stable",
        check_for_updates: () => updateAvailable,
        install_update: () => undefined,
      }),
    );
    const { findByRole, findByText } = render(() => <UpdatesPanel />);
    await flushAll();
    fireEvent.click(await findByRole("button", { name: "Check now" }));
    await flushAll();
    fireEvent.click(await findByRole("button", { name: "Install" }));
    await flushAll();
    expect(invokeMock).toHaveBeenCalledWith("install_update", { channel: "stable" });
    expect(await findByText(/Update installed: 0\.3\.0/)).toBeDefined();
    expect(await findByRole("button", { name: "Restart Stint" })).toBeDefined();
  });

  it("Install surfaces an error message when the IPC rejects", async () => {
    invokeMock.mockImplementation(
      makeInvokeImpl({
        settings_get: () => "stable",
        check_for_updates: () => updateAvailable,
        install_update: () => {
          throw new Error("install boom");
        },
      }),
    );
    const { findByRole, findByText } = render(() => <UpdatesPanel />);
    await flushAll();
    fireEvent.click(await findByRole("button", { name: "Check now" }));
    await flushAll();
    fireEvent.click(await findByRole("button", { name: "Install" }));
    await flushAll();
    expect(await findByText("install boom")).toBeDefined();
  });

  it("Restart Stint after install calls restart_app", async () => {
    invokeMock.mockImplementation(
      makeInvokeImpl({
        settings_get: () => "stable",
        check_for_updates: () => updateAvailable,
        install_update: () => undefined,
        restart_app: () => undefined,
      }),
    );
    const { findByRole } = render(() => <UpdatesPanel />);
    await flushAll();
    fireEvent.click(await findByRole("button", { name: "Check now" }));
    await flushAll();
    fireEvent.click(await findByRole("button", { name: "Install" }));
    await flushAll();
    fireEvent.click(await findByRole("button", { name: "Restart Stint" }));
    await flushAll();
    expect(invokeMock).toHaveBeenCalledWith("restart_app");
  });

  it("Restart Stint surfaces an error when restart_app rejects", async () => {
    invokeMock.mockImplementation(
      makeInvokeImpl({
        settings_get: () => "stable",
        check_for_updates: () => updateAvailable,
        install_update: () => undefined,
        restart_app: () => {
          throw new Error("no restart");
        },
      }),
    );
    const { findByRole, findByText } = render(() => <UpdatesPanel />);
    await flushAll();
    fireEvent.click(await findByRole("button", { name: "Check now" }));
    await flushAll();
    fireEvent.click(await findByRole("button", { name: "Install" }));
    await flushAll();
    fireEvent.click(await findByRole("button", { name: "Restart Stint" }));
    await flushAll();
    expect(await findByText("no restart")).toBeDefined();
  });

  it("switching the channel persists via settings_set and shows the beta warning", async () => {
    let currentChannel = "stable";
    invokeMock.mockImplementation((cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "settings_get") return Promise.resolve(currentChannel);
      if (cmd === "settings_set") {
        currentChannel = String(args?.value);
        return Promise.resolve(undefined);
      }
      return Promise.reject(new Error(`unexpected: ${cmd}`));
    });
    const { getByRole, findByText } = render(() => <UpdatesPanel />);
    await flushAll();
    const select = getByRole("combobox") as HTMLSelectElement;
    fireEvent.change(select, { target: { value: "beta" } });
    await flushAll();
    expect(invokeMock).toHaveBeenCalledWith("settings_set", {
      key: "update.channel",
      value: "beta",
    });
    expect(await findByText(/Switching back to Stable won't downgrade you/)).toBeDefined();
  });

  it("switching the channel surfaces an error when settings_set rejects", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "settings_get") return Promise.resolve("stable");
      if (cmd === "settings_set") return Promise.reject(new Error("write fail"));
      return Promise.reject(new Error(`unexpected: ${cmd}`));
    });
    const { getByRole, findByText } = render(() => <UpdatesPanel />);
    await flushAll();
    const select = getByRole("combobox") as HTMLSelectElement;
    fireEvent.change(select, { target: { value: "beta" } });
    await flushAll();
    expect(await findByText("write fail")).toBeDefined();
  });

  it("Check now button shows 'Checking…' while the call is in flight", async () => {
    let resolveCheck!: (v: UpdateInfo) => void;
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "settings_get") return Promise.resolve("stable");
      if (cmd === "check_for_updates") {
        return new Promise<UpdateInfo>((r) => {
          resolveCheck = r;
        });
      }
      return Promise.reject(new Error(`unexpected: ${cmd}`));
    });
    const { findByRole } = render(() => <UpdatesPanel />);
    await flushAll();
    fireEvent.click(await findByRole("button", { name: "Check now" }));
    // Microtask flush so the signal setter for `checking` runs.
    await flushMicrotasks();
    expect(await findByRole("button", { name: "Checking…" })).toBeDefined();
    resolveCheck(upToDate);
    await flushAll();
    expect(await findByRole("button", { name: "Check now" })).toBeDefined();
  });

  // NOTE: kept last because `requestCheckForUpdates` increments a module-level
  // counter; once bumped, subsequent <UpdatesPanel> mounts in the same test
  // file will also auto-fire a check on mount.
  it("requestCheckForUpdates fires a check automatically when the panel mounts", async () => {
    invokeMock.mockImplementation(
      makeInvokeImpl({
        settings_get: () => "stable",
        check_for_updates: () => upToDate,
      }),
    );
    requestCheckForUpdates();
    const { findByText } = render(() => <UpdatesPanel />);
    await flushAll();
    expect(invokeMock).toHaveBeenCalledWith("check_for_updates", { channel: "stable" });
    expect(await findByText(/You're on the latest version/)).toBeDefined();
  });
});
