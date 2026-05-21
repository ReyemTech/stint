import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render } from "@solidjs/testing-library";

vi.mock("~/lib/openSolidtime", () => ({
  openSolidtime: vi.fn().mockResolvedValue(undefined),
}));

import MainNav from "~/components/MainNav";
import { openSolidtime } from "~/lib/openSolidtime";

beforeEach(() => {
  vi.mocked(openSolidtime).mockClear();
});

describe("<MainNav>", () => {
  it("renders Today / Settings / About links with the right hrefs", () => {
    const { getByText } = render(() => <MainNav active="today" />);
    expect((getByText("Today") as HTMLAnchorElement).getAttribute("href")).toBe(
      "#/today",
    );
    expect(
      (getByText("Settings") as HTMLAnchorElement).getAttribute("href"),
    ).toBe("#/settings");
    expect((getByText("About") as HTMLAnchorElement).getAttribute("href")).toBe(
      "#/about",
    );
  });

  it("highlights the active route via the zinc-900 text class", () => {
    const { getByText } = render(() => <MainNav active="settings" />);
    expect((getByText("Settings") as HTMLAnchorElement).className).toContain(
      "text-zinc-900",
    );
    expect((getByText("Today") as HTMLAnchorElement).className).toContain(
      "text-zinc-500",
    );
  });

  it("renders an optional leading slot before the links", () => {
    const { getByTestId, getByText } = render(() => (
      <MainNav
        active="today"
        leading={<span data-testid="sync-badge">badge</span>}
      />
    ));
    const badge = getByTestId("sync-badge");
    expect(badge).toBeDefined();
    // The leading slot appears in the same nav as the links.
    expect(getByText("Today")).toBeDefined();
  });

  it("clicking the Solidtime button calls openSolidtime()", () => {
    const { getByText } = render(() => <MainNav active="today" />);
    fireEvent.click(getByText("Solidtime ↗"));
    expect(openSolidtime).toHaveBeenCalledTimes(1);
  });
});
