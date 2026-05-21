import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render } from "@solidjs/testing-library";

vi.mock("@tauri-apps/api/app", () => ({
  getVersion: vi.fn().mockResolvedValue("0.1.0"),
  getTauriVersion: vi.fn().mockResolvedValue("2.1.0"),
}));
vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn().mockResolvedValue(undefined),
}));
vi.mock("~/lib/openSolidtime", () => ({
  openSolidtime: vi.fn().mockResolvedValue(undefined),
}));

import About from "~/routes/About";
import { openUrl } from "@tauri-apps/plugin-opener";
import { openSolidtime } from "~/lib/openSolidtime";

const flushMicrotasks = () => new Promise<void>((r) => setTimeout(r, 0));

beforeEach(() => {
  vi.mocked(openUrl).mockClear();
  vi.mocked(openSolidtime).mockClear();
});

describe("<About>", () => {
  it("renders the heading and the identity card", async () => {
    const { getByRole, getByText } = render(() => <About />);
    await flushMicrotasks();
    expect(getByRole("heading", { name: "About", level: 1 })).toBeDefined();
    expect(getByText("Stint")).toBeDefined();
  });

  it("displays the resolved app version and tauri version", async () => {
    const { findByText } = render(() => <About />);
    expect(await findByText("0.1.0")).toBeDefined();
    expect(await findByText("2.1.0")).toBeDefined();
  });

  it("Open Solidtime button calls openSolidtime()", async () => {
    const { getByText } = render(() => <About />);
    await flushMicrotasks();
    fireEvent.click(getByText("Open Solidtime"));
    expect(openSolidtime).toHaveBeenCalledTimes(1);
  });

  it("GitHub + Report buttons each call openUrl", async () => {
    const { getByText } = render(() => <About />);
    await flushMicrotasks();
    fireEvent.click(getByText("GitHub"));
    fireEvent.click(getByText("Report an issue"));
    expect(openUrl).toHaveBeenNthCalledWith(1, "https://github.com/reyemtech/stint");
    expect(openUrl).toHaveBeenNthCalledWith(
      2,
      "https://github.com/reyemtech/stint/issues",
    );
  });

  it("Reyem Tech credit button calls openUrl with the homepage", async () => {
    const { getByText } = render(() => <About />);
    await flushMicrotasks();
    fireEvent.click(getByText("Reyem Tech ↗"));
    expect(openUrl).toHaveBeenCalledWith("https://www.reyem.tech");
  });

  it("each Built-with row's Visit ↗ button calls openUrl with the credit's URL", async () => {
    const { getAllByText } = render(() => <About />);
    await flushMicrotasks();
    const visitBtns = getAllByText("Visit ↗");
    expect(visitBtns.length).toBe(6); // 6 CREDITS
    fireEvent.click(visitBtns[0]);
    expect(openUrl).toHaveBeenCalledWith("https://tauri.app");
  });
});
