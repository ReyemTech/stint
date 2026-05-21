import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render } from "@solidjs/testing-library";

vi.mock("~/api", () => ({
  api: {
    configShow: vi.fn(),
    configSet: vi.fn().mockResolvedValue(undefined),
    configTest: vi.fn(),
    listOrganizations: vi.fn(),
    listProjects: vi.fn(),
    refreshProjects: vi.fn(),
    syncNow: vi.fn(),
  },
  calendarApi: {
    listAccounts: vi.fn(),
    addGoogle: vi.fn(),
    removeAccount: vi.fn().mockResolvedValue(undefined),
    listCalendars: vi.fn().mockResolvedValue([]),
    setCalendarIncluded: vi.fn(),
    setDefaultProject: vi.fn().mockResolvedValue(undefined),
    oauthStatus: vi.fn().mockResolvedValue({ signed_in: true, scope: null }),
  },
  oauthSolidtimeStatus: vi.fn(),
  oauthSolidtimeStart: vi.fn().mockResolvedValue(undefined),
  oauthSolidtimeLogout: vi.fn().mockResolvedValue(undefined),
}));
vi.mock("~/lib/openSolidtime", () => ({
  openSolidtime: vi.fn().mockResolvedValue(undefined),
}));

import Settings from "~/routes/Settings";
import {
  api,
  calendarApi,
  oauthSolidtimeLogout,
  oauthSolidtimeStart,
  oauthSolidtimeStatus,
} from "~/api";

const flushMicrotasks = () => new Promise<void>((r) => setTimeout(r, 0));
async function flushAll() {
  for (let i = 0; i < 5; i++) await flushMicrotasks();
}

beforeEach(() => {
  vi.mocked(api.configShow).mockReset().mockResolvedValue([]);
  vi.mocked(api.configSet).mockClear();
  vi.mocked(api.configTest).mockReset().mockResolvedValue("me@example.com");
  vi.mocked(api.listOrganizations).mockReset().mockResolvedValue([]);
  vi.mocked(api.listProjects).mockReset().mockResolvedValue([]);
  vi.mocked(api.refreshProjects).mockReset().mockResolvedValue(0);
  vi.mocked(api.syncNow).mockReset().mockResolvedValue(2);
  vi.mocked(calendarApi.listAccounts).mockReset().mockResolvedValue([]);
  vi.mocked(calendarApi.addGoogle).mockReset();
  vi.mocked(calendarApi.removeAccount).mockClear();
  vi.mocked(oauthSolidtimeStatus)
    .mockReset()
    .mockResolvedValue({ mode: "api_token", signed_in: false, scope: null });
  vi.mocked(oauthSolidtimeStart).mockClear();
  vi.mocked(oauthSolidtimeLogout).mockClear();
});

describe("<Settings>", () => {
  it("renders the heading and both accordions", async () => {
    const { getByRole, findByRole } = render(() => <Settings />);
    expect(getByRole("heading", { name: "Settings", level: 1 })).toBeDefined();
    expect(await findByRole("button", { name: /Solidtime connection/ })).toBeDefined();
    expect(await findByRole("button", { name: /Calendar accounts/ })).toBeDefined();
  });

  it("Test connection button calls api.configTest and surfaces success", async () => {
    const { findByRole, findByText } = render(() => <Settings />);
    await flushAll();
    // The Solidtime connection accordion needs to be open.
    fireEvent.click(await findByRole("button", { name: /Solidtime connection/ }));
    fireEvent.click(await findByRole("button", { name: "Test connection" }));
    await flushAll();
    expect(api.configTest).toHaveBeenCalled();
    expect(await findByText(/Connected as me@example.com/)).toBeDefined();
  });

  it("Sync now button calls api.syncNow and surfaces item count", async () => {
    const { findByRole, findByText } = render(() => <Settings />);
    await flushAll();
    fireEvent.click(await findByRole("button", { name: /Solidtime connection/ }));
    fireEvent.click(await findByRole("button", { name: "Sync now" }));
    await flushAll();
    expect(api.syncNow).toHaveBeenCalled();
    expect(await findByText(/Synced 2 items/)).toBeDefined();
  });

  it("Sync now surfaces an error message when syncNow throws", async () => {
    vi.mocked(api.syncNow).mockRejectedValue(new Error("nope"));
    const { findByRole, findByText } = render(() => <Settings />);
    await flushAll();
    fireEvent.click(await findByRole("button", { name: /Solidtime connection/ }));
    fireEvent.click(await findByRole("button", { name: "Sync now" }));
    await flushAll();
    expect(await findByText(/nope/)).toBeDefined();
  });

  it("clicking 'No calendar accounts' empty state then 'Add Google account' fires calendarApi.addGoogle", async () => {
    vi.mocked(calendarApi.addGoogle).mockResolvedValue({
      id: "acc-1",
      provider: "google",
      display_name: "Me",
      identifier: "me@example.com",
      caldav_url: null,
      enabled: true,
      created_at: "2026-05-20T00:00:00Z",
    });
    const { findByRole, findByText } = render(() => <Settings />);
    await flushAll();
    fireEvent.click(await findByRole("button", { name: /Calendar accounts/ }));
    expect(await findByText(/No calendar accounts/)).toBeDefined();
    fireEvent.click(await findByRole("button", { name: "Add Google account" }));
    await flushAll();
    expect(calendarApi.addGoogle).toHaveBeenCalled();
  });

  it("renders a calendar account row when listAccounts returns one", async () => {
    vi.mocked(calendarApi.listAccounts).mockResolvedValue([
      {
        id: "acc-1",
        provider: "google",
        display_name: "Me",
        identifier: "me@example.com",
        caldav_url: null,
        enabled: true,
        created_at: "2026-05-20T00:00:00Z",
      },
    ]);
    const { findByRole, findByText } = render(() => <Settings />);
    await flushAll();
    fireEvent.click(await findByRole("button", { name: /Calendar accounts/ }));
    expect(await findByText("me@example.com")).toBeDefined();
  });

  it("Remove on a calendar account row calls calendarApi.removeAccount", async () => {
    vi.mocked(calendarApi.listAccounts).mockResolvedValue([
      {
        id: "acc-1",
        provider: "google",
        display_name: "Me",
        identifier: "me@example.com",
        caldav_url: null,
        enabled: true,
        created_at: "2026-05-20T00:00:00Z",
      },
    ]);
    const { findByRole } = render(() => <Settings />);
    await flushAll();
    fireEvent.click(await findByRole("button", { name: /Calendar accounts/ }));
    fireEvent.click(await findByRole("button", { name: "Remove" }));
    await flushAll();
    expect(calendarApi.removeAccount).toHaveBeenCalledWith("acc-1");
  });

  it("switching to OAuth auth method reveals Sign in button when not signed in", async () => {
    vi.mocked(oauthSolidtimeStatus).mockResolvedValue({
      mode: "api_token",
      signed_in: false,
      scope: null,
    });
    const { findByRole, container } = render(() => <Settings />);
    await flushAll();
    fireEvent.click(await findByRole("button", { name: /Solidtime connection/ }));
    const oauthRadio = container.querySelector(
      'input[type="radio"][value="oauth"]',
    ) as HTMLInputElement;
    fireEvent.click(oauthRadio);
    await flushAll();
    const signIn = await findByRole("button", { name: "Sign in with Solidtime" });
    fireEvent.click(signIn);
    await flushAll();
    expect(oauthSolidtimeStart).toHaveBeenCalled();
  });

  it("OAuth with signed_in=true shows Sign out, which calls oauthSolidtimeLogout", async () => {
    vi.mocked(oauthSolidtimeStatus).mockResolvedValue({
      mode: "oauth",
      signed_in: true,
      scope: "read",
    });
    const { findByRole } = render(() => <Settings />);
    await flushAll();
    fireEvent.click(await findByRole("button", { name: /Solidtime connection/ }));
    await flushAll();
    fireEvent.click(await findByRole("button", { name: "Sign out" }));
    await flushAll();
    expect(oauthSolidtimeLogout).toHaveBeenCalled();
  });

  it("shows an organization dropdown when memberships have loaded", async () => {
    vi.mocked(api.configShow).mockResolvedValue([
      { key: "solidtime.url", value: "https://example.com", is_secret: false, present: true },
      { key: "solidtime.token", value: null, is_secret: true, present: true },
    ]);
    vi.mocked(api.listOrganizations).mockResolvedValue([
      { id: "org-1", member_id: "m-1", name: "Acme" },
      { id: "org-2", member_id: "m-2", name: "Beta" },
    ]);
    const { findByRole, findByText } = render(() => <Settings />);
    await flushAll();
    fireEvent.click(await findByRole("button", { name: /Solidtime connection/ }));
    await flushAll();
    expect(await findByText("Acme")).toBeDefined();
    expect(await findByText("Beta")).toBeDefined();
  });
});
