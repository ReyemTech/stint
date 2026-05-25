import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render } from "@solidjs/testing-library";

// Mock the Tauri IPC layer. IntegrationsPanel calls `invoke` directly
// (without going through `~/api`) so we mock the underlying core module.
const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn().mockResolvedValue(undefined),
}));

import IntegrationsPanel from "~/routes/Settings/IntegrationsPanel";
import { openUrl } from "@tauri-apps/plugin-opener";

const flushMicrotasks = () => new Promise<void>((r) => setTimeout(r, 0));
async function flushAll() {
  for (let i = 0; i < 5; i++) await flushMicrotasks();
}

type ApiState = {
  enabled: boolean;
  host: string;
  port: number | null;
  base_url: string | null;
  bound_this_session: boolean;
};

const disabledState: ApiState = {
  enabled: false,
  host: "127.0.0.1",
  port: null,
  base_url: null,
  bound_this_session: false,
};

const enabledBoundState: ApiState = {
  enabled: true,
  host: "127.0.0.1",
  port: 4321,
  base_url: "http://127.0.0.1:4321",
  bound_this_session: true,
};

const enabledUnboundState: ApiState = {
  enabled: true,
  host: "127.0.0.1",
  port: null,
  base_url: null,
  bound_this_session: false,
};

beforeEach(() => {
  invokeMock.mockReset();
  vi.mocked(openUrl).mockClear();
});

describe("<IntegrationsPanel>", () => {
  it("loads initial state from get_api_integration_state and shows Disabled", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "get_api_integration_state") return Promise.resolve(disabledState);
      return Promise.reject(new Error(`unexpected ${cmd}`));
    });
    const { findByRole, queryByText } = render(() => <IntegrationsPanel />);
    await flushAll();
    expect(invokeMock).toHaveBeenCalledWith("get_api_integration_state");
    // Toggle reflects disabled state (label is "Disabled")
    const toggle = await findByRole("switch");
    expect(toggle.getAttribute("aria-checked")).toBe("false");
    expect(toggle.textContent).toContain("Disabled");
    // No success card while disabled
    expect(queryByText("URL")).toBeNull();
  });

  it("renders the host/port/url card when enabled AND bound_this_session", async () => {
    invokeMock.mockImplementation((cmd: string) =>
      cmd === "get_api_integration_state"
        ? Promise.resolve(enabledBoundState)
        : Promise.reject(new Error(`unexpected ${cmd}`)),
    );
    const { findByText } = render(() => <IntegrationsPanel />);
    await flushAll();
    expect(await findByText("127.0.0.1")).toBeDefined();
    expect(await findByText("4321")).toBeDefined();
    expect(await findByText("http://127.0.0.1:4321")).toBeDefined();
  });

  it("shows 'Restart Stint to start' pending message when toggling on while disabled", async () => {
    // First load: disabled. After set_api_enabled(true), refetch returns enabled+unbound.
    invokeMock
      .mockResolvedValueOnce(disabledState) // initial load
      .mockResolvedValueOnce(enabledUnboundState) // set_api_enabled
      .mockResolvedValueOnce(enabledUnboundState); // refetch

    const { findByRole, findByText } = render(() => <IntegrationsPanel />);
    await flushAll();
    fireEvent.click(await findByRole("switch"));
    await flushAll();
    expect(invokeMock).toHaveBeenCalledWith("set_api_enabled", { enabled: true });
    expect(await findByText(/Restart Stint to start the API server/)).toBeDefined();
  });

  it("shows 'Restart Stint to stop' pending message when toggling off while bound", async () => {
    // Bound on initial load (enabled+bound). Toggle off → enabled=false but bound_this_session still true.
    const stillBoundButDisabled: ApiState = {
      ...enabledBoundState,
      enabled: false,
    };
    invokeMock
      .mockResolvedValueOnce(enabledBoundState)
      .mockResolvedValueOnce(stillBoundButDisabled)
      .mockResolvedValueOnce(stillBoundButDisabled);

    const { findByRole, findByText } = render(() => <IntegrationsPanel />);
    await flushAll();
    fireEvent.click(await findByRole("switch"));
    await flushAll();
    expect(invokeMock).toHaveBeenCalledWith("set_api_enabled", { enabled: false });
    expect(await findByText(/Restart Stint to stop the API server/)).toBeDefined();
  });

  it("surfaces an error when set_api_enabled rejects", async () => {
    invokeMock
      .mockResolvedValueOnce(disabledState) // initial load
      .mockRejectedValueOnce(new Error("boom"));

    const { findByRole, findByText } = render(() => <IntegrationsPanel />);
    await flushAll();
    fireEvent.click(await findByRole("switch"));
    await flushAll();
    expect(await findByText("boom")).toBeDefined();
  });

  it("Copy button writes base_url to the clipboard and flips label to 'Copied'", async () => {
    invokeMock.mockResolvedValue(enabledBoundState);
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    const { findByRole } = render(() => <IntegrationsPanel />);
    await flushAll();
    const copyBtn = await findByRole("button", { name: "Copy" });
    fireEvent.click(copyBtn);
    await flushAll();
    expect(writeText).toHaveBeenCalledWith("http://127.0.0.1:4321");
    // The label flips to "Copied" while the timeout is pending.
    expect((await findByRole("button", { name: "Copied" })).textContent).toBe("Copied");
  });

  it("surfaces a clipboard error when writeText rejects", async () => {
    invokeMock.mockResolvedValue(enabledBoundState);
    const writeText = vi.fn().mockRejectedValue(new Error("clipboard nope"));
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    const { findByRole, findByText } = render(() => <IntegrationsPanel />);
    await flushAll();
    fireEvent.click(await findByRole("button", { name: "Copy" }));
    await flushAll();
    expect(await findByText("clipboard nope")).toBeDefined();
  });

  it("Learn more link calls openUrl with the AI docs URL", async () => {
    invokeMock.mockResolvedValue(disabledState);
    const { findByRole } = render(() => <IntegrationsPanel />);
    await flushAll();
    fireEvent.click(await findByRole("button", { name: /Learn more/ }));
    await flushAll();
    expect(openUrl).toHaveBeenCalledWith("https://stint.reyem.tech/ai-integration/");
  });

  it("surfaces an error when openUrl rejects on Learn more", async () => {
    invokeMock.mockResolvedValue(disabledState);
    vi.mocked(openUrl).mockRejectedValueOnce(new Error("opener busted"));
    const { findByRole, findByText } = render(() => <IntegrationsPanel />);
    await flushAll();
    fireEvent.click(await findByRole("button", { name: /Learn more/ }));
    await flushAll();
    expect(await findByText("opener busted")).toBeDefined();
  });

  it("Test stint://current button calls openUrl with the URL scheme", async () => {
    invokeMock.mockResolvedValue(disabledState);
    const { findByRole } = render(() => <IntegrationsPanel />);
    await flushAll();
    fireEvent.click(await findByRole("button", { name: "Test stint://current" }));
    await flushAll();
    expect(openUrl).toHaveBeenCalledWith("stint://current");
  });

  it("surfaces an error when openUrl rejects on the URL-scheme test button", async () => {
    invokeMock.mockResolvedValue(disabledState);
    vi.mocked(openUrl).mockRejectedValueOnce(new Error("scheme broken"));
    const { findByRole, findByText } = render(() => <IntegrationsPanel />);
    await flushAll();
    fireEvent.click(await findByRole("button", { name: "Test stint://current" }));
    await flushAll();
    expect(await findByText("scheme broken")).toBeDefined();
  });

  it("renders the AI agents install snippet and all four stint:// URL variants", async () => {
    invokeMock.mockResolvedValue(disabledState);
    const { findByText, getByText } = render(() => <IntegrationsPanel />);
    await flushAll();
    expect(
      await findByText(/stint skill install <claude\|codex\|opencode>/),
    ).toBeDefined();
    expect(getByText(/stint:\/\/start\?description=/)).toBeDefined();
    expect(getByText("stint://stop")).toBeDefined();
    expect(getByText("stint://current")).toBeDefined();
    expect(getByText(/stint:\/\/entry\/<local-uuid>/)).toBeDefined();
  });

  it("shows the loading fallback before the resource resolves", async () => {
    let resolveFn!: (s: ApiState) => void;
    invokeMock.mockReturnValueOnce(
      new Promise<ApiState>((r) => {
        resolveFn = r;
      }),
    );
    const { getByText } = render(() => <IntegrationsPanel />);
    expect(getByText("Loading…")).toBeDefined();
    resolveFn(disabledState);
    await flushAll();
  });

  it("does not render the bound info card when enabled but bound_this_session is false", async () => {
    invokeMock.mockResolvedValue(enabledUnboundState);
    const { findByRole, queryByText, findByText } = render(() => <IntegrationsPanel />);
    await flushAll();
    // Toggle reports Enabled state
    expect((await findByRole("switch")).getAttribute("aria-checked")).toBe("true");
    // No "URL" dl row because bound_this_session is false
    expect(queryByText("URL")).toBeNull();
    // The "needs restart" hint is visible (enabled !== bound_this_session).
    expect(await findByText(/Restart Stint to start the API server/)).toBeDefined();
  });
});
