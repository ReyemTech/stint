import { describe, expect, it, vi, beforeEach } from "vitest";
import { render } from "@solidjs/testing-library";

// App.tsx reads getCurrentWindow().label at module-load time, so the
// window mock must be in place before the import statement. vi.mock is
// hoisted above all imports, so we inline the return value (no outside
// const refs allowed in the factory).
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    label: "main",
    hide: () => Promise.resolve(),
  }),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

// Stub the routes so App.tsx renders with no cascading deps.
vi.mock("~/routes/Today", () => ({
  default: () => <div data-testid="today-route">today</div>,
}));
vi.mock("~/routes/Settings", () => ({
  default: () => <div data-testid="settings-route">settings</div>,
}));
vi.mock("~/routes/About", () => ({
  default: () => <div data-testid="about-route">about</div>,
}));
vi.mock("~/routes/Popover", () => ({
  default: () => <div data-testid="popover-route">popover</div>,
}));

import App from "~/App";
import { listen } from "@tauri-apps/api/event";

beforeEach(() => {
  vi.mocked(listen).mockClear();
  // Reset hash before each test so routes resolve deterministically.
  window.location.hash = "";
});

describe("<App>", () => {
  it("renders the Today route by default (catch-all routes to Today)", async () => {
    const { findByTestId } = render(() => <App />);
    expect(await findByTestId("today-route")).toBeDefined();
  });

  it("renders the Settings route when the hash navigates to /settings", async () => {
    window.location.hash = "#/settings";
    const { findByTestId } = render(() => <App />);
    expect(await findByTestId("settings-route")).toBeDefined();
  });

  it("renders the About route when the hash navigates to /about", async () => {
    window.location.hash = "#/about";
    const { findByTestId } = render(() => <App />);
    expect(await findByTestId("about-route")).toBeDefined();
  });
});
