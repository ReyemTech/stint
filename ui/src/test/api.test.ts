import { describe, expect, it, vi, beforeEach } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import {
  api,
  calendarApi,
  conflictResolve,
  oauthSolidtimeLogout,
  oauthSolidtimeStart,
  oauthSolidtimeStatus,
  pullNow,
} from "~/api";

const mockInvoke = vi.mocked(invoke);

beforeEach(() => {
  mockInvoke.mockReset();
  mockInvoke.mockResolvedValue(undefined as unknown as never);
});

describe("api (timer commands)", () => {
  it("getRunningTimer invokes get_running_timer with no args", async () => {
    await api.getRunningTimer();
    expect(mockInvoke).toHaveBeenCalledWith("get_running_timer");
  });

  it("startTimer wraps args in { args: { description, project_id, task_id, billable, start_at } }", async () => {
    await api.startTimer("write tests", "p-1", "t-1", true, "2026-05-20T08:00:00Z");
    expect(mockInvoke).toHaveBeenCalledWith("start_timer", {
      args: {
        description: "write tests",
        project_id: "p-1",
        task_id: "t-1",
        billable: true,
        start_at: "2026-05-20T08:00:00Z",
      },
    });
  });

  it("startTimer defaults optional args to null/false", async () => {
    await api.startTimer("solo");
    expect(mockInvoke).toHaveBeenCalledWith("start_timer", {
      args: {
        description: "solo",
        project_id: null,
        task_id: null,
        billable: false,
        start_at: null,
      },
    });
  });

  it("stopTimer invokes stop_timer with no args", async () => {
    await api.stopTimer();
    expect(mockInvoke).toHaveBeenCalledWith("stop_timer");
  });

  it("deleteEntry passes localUuid through", async () => {
    await api.deleteEntry("uuid-1");
    expect(mockInvoke).toHaveBeenCalledWith("delete_entry", {
      localUuid: "uuid-1",
    });
  });

  it("updateDescription passes localUuid + description", async () => {
    await api.updateDescription("uuid-1", "new desc");
    expect(mockInvoke).toHaveBeenCalledWith("update_description", {
      localUuid: "uuid-1",
      description: "new desc",
    });
  });

  it("setEntryProject accepts null projectId", async () => {
    await api.setEntryProject("uuid-1", null);
    expect(mockInvoke).toHaveBeenCalledWith("set_entry_project", {
      localUuid: "uuid-1",
      projectId: null,
    });
  });

  it("setEntryProject accepts a project id", async () => {
    await api.setEntryProject("uuid-1", "p-42");
    expect(mockInvoke).toHaveBeenCalledWith("set_entry_project", {
      localUuid: "uuid-1",
      projectId: "p-42",
    });
  });

  it("setEntryBillable passes billable bool", async () => {
    await api.setEntryBillable("uuid-1", true);
    expect(mockInvoke).toHaveBeenCalledWith("set_entry_billable", {
      localUuid: "uuid-1",
      billable: true,
    });
  });

  it("restartEntry passes localUuid", async () => {
    await api.restartEntry("uuid-99");
    expect(mockInvoke).toHaveBeenCalledWith("restart_entry", {
      localUuid: "uuid-99",
    });
  });
});

describe("api (entries / projects / config / sync)", () => {
  it("listToday invokes list_today", async () => {
    await api.listToday();
    expect(mockInvoke).toHaveBeenCalledWith("list_today");
  });

  it("listBetween passes from/to", async () => {
    await api.listBetween("2026-05-20T00:00:00Z", "2026-05-21T00:00:00Z");
    expect(mockInvoke).toHaveBeenCalledWith("list_between", {
      from: "2026-05-20T00:00:00Z",
      to: "2026-05-21T00:00:00Z",
    });
  });

  it("listProjects / refreshProjects / listOrganizations have no args", async () => {
    await api.listProjects();
    await api.refreshProjects();
    await api.listOrganizations();
    expect(mockInvoke).toHaveBeenNthCalledWith(1, "list_projects");
    expect(mockInvoke).toHaveBeenNthCalledWith(2, "refresh_projects");
    expect(mockInvoke).toHaveBeenNthCalledWith(3, "list_organizations");
  });

  it("configShow / configTest / solidtimeUrl have no args", async () => {
    await api.configShow();
    await api.configTest();
    await api.solidtimeUrl();
    expect(mockInvoke).toHaveBeenNthCalledWith(1, "config_show");
    expect(mockInvoke).toHaveBeenNthCalledWith(2, "config_test");
    expect(mockInvoke).toHaveBeenNthCalledWith(3, "solidtime_url");
  });

  it("configSet passes key+value", async () => {
    await api.configSet("solidtime.url", "https://example.com");
    expect(mockInvoke).toHaveBeenCalledWith("config_set", {
      key: "solidtime.url",
      value: "https://example.com",
    });
  });

  it("syncNow invokes sync_now", async () => {
    await api.syncNow();
    expect(mockInvoke).toHaveBeenCalledWith("sync_now");
  });
});

describe("oauth helpers", () => {
  it("oauthSolidtimeStatus invokes oauth_solidtime_status", async () => {
    await oauthSolidtimeStatus();
    expect(mockInvoke).toHaveBeenCalledWith("oauth_solidtime_status");
  });
  it("oauthSolidtimeStart invokes oauth_solidtime_start", async () => {
    await oauthSolidtimeStart();
    expect(mockInvoke).toHaveBeenCalledWith("oauth_solidtime_start");
  });
  it("oauthSolidtimeLogout invokes oauth_solidtime_logout", async () => {
    await oauthSolidtimeLogout();
    expect(mockInvoke).toHaveBeenCalledWith("oauth_solidtime_logout");
  });
});

describe("pull / conflict_resolve", () => {
  it("pullNow invokes pull_now with no args", async () => {
    await pullNow();
    expect(mockInvoke).toHaveBeenCalledWith("pull_now");
  });

  it("conflictResolve wraps args in { args: { action, remote_id } }", async () => {
    await conflictResolve("stop_remote", "remote-1");
    expect(mockInvoke).toHaveBeenCalledWith("conflict_resolve", {
      args: { action: "stop_remote", remote_id: "remote-1" },
    });
    await conflictResolve("switch", "remote-2");
    await conflictResolve("dismiss", "remote-3");
    expect(mockInvoke).toHaveBeenCalledTimes(3);
  });
});

describe("calendarApi", () => {
  it("listAccounts / addGoogle have no args", async () => {
    await calendarApi.listAccounts();
    await calendarApi.addGoogle();
    expect(mockInvoke).toHaveBeenNthCalledWith(1, "calendar_list_accounts");
    expect(mockInvoke).toHaveBeenNthCalledWith(2, "calendar_add_google");
  });

  it("oauthStatus / removeAccount / listCalendars / refreshAccount pass accountId", async () => {
    await calendarApi.oauthStatus("acc-1");
    await calendarApi.removeAccount("acc-2");
    await calendarApi.listCalendars("acc-3");
    await calendarApi.refreshAccount("acc-4");
    expect(mockInvoke).toHaveBeenNthCalledWith(1, "calendar_oauth_status", { accountId: "acc-1" });
    expect(mockInvoke).toHaveBeenNthCalledWith(2, "calendar_remove_account", { accountId: "acc-2" });
    expect(mockInvoke).toHaveBeenNthCalledWith(3, "calendar_list_calendars", { accountId: "acc-3" });
    expect(mockInvoke).toHaveBeenNthCalledWith(4, "calendar_refresh_account", { accountId: "acc-4" });
  });

  it("setCalendarIncluded passes calendarId + included", async () => {
    await calendarApi.setCalendarIncluded("cal-1", true);
    expect(mockInvoke).toHaveBeenCalledWith("calendar_set_calendar_included", {
      calendarId: "cal-1",
      included: true,
    });
  });

  it("listEventsInRange passes accountId/from/to", async () => {
    await calendarApi.listEventsInRange("acc-1", "2026-05-20T00:00:00Z", "2026-05-21T00:00:00Z");
    expect(mockInvoke).toHaveBeenCalledWith("calendar_list_events_in_range", {
      accountId: "acc-1",
      from: "2026-05-20T00:00:00Z",
      to: "2026-05-21T00:00:00Z",
    });
  });

  it("logEvent + ignoreEvent pass accountId/eventId/eventStart", async () => {
    await calendarApi.logEvent("acc-1", "evt-1", "2026-05-20T09:00:00Z");
    await calendarApi.ignoreEvent("acc-1", "evt-2", "2026-05-20T10:00:00Z");
    expect(mockInvoke).toHaveBeenNthCalledWith(1, "calendar_log_event", {
      accountId: "acc-1",
      eventId: "evt-1",
      eventStart: "2026-05-20T09:00:00Z",
    });
    expect(mockInvoke).toHaveBeenNthCalledWith(2, "calendar_ignore_event", {
      accountId: "acc-1",
      eventId: "evt-2",
      eventStart: "2026-05-20T10:00:00Z",
    });
  });

  it("revertEvent passes accountId/eventId/eventStart", async () => {
    await calendarApi.revertEvent("acc-1", "evt-3", "2026-05-20T11:00:00Z");
    expect(mockInvoke).toHaveBeenCalledWith("calendar_revert_event", {
      accountId: "acc-1",
      eventId: "evt-3",
      eventStart: "2026-05-20T11:00:00Z",
    });
  });
});
