import { invoke } from "@tauri-apps/api/core";
import type {
  CalendarAccount,
  CalendarEventWithDecision,
  CalendarOAuthStatus,
  CalendarRow,
  ConfigEntry,
  Entry,
  OrgChoice,
  Project,
  RunningTimer,
} from "./types";

export const api = {
  getRunningTimer: () => invoke<RunningTimer | null>("get_running_timer"),
  startTimer: (
    description: string,
    projectId?: string | null,
    taskId?: string | null,
    billable = false,
    startAt?: string | null,
  ) =>
    invoke<string>("start_timer", {
      args: {
        description,
        project_id: projectId ?? null,
        task_id: taskId ?? null,
        billable,
        start_at: startAt ?? null,
      },
    }),
  stopTimer: () => invoke<string>("stop_timer"),
  deleteEntry: (localUuid: string) =>
    invoke<void>("delete_entry", { localUuid }),
  updateDescription: (localUuid: string, description: string) =>
    invoke<void>("update_description", { localUuid, description }),
  setEntryProject: (localUuid: string, projectId: string | null) =>
    invoke<void>("set_entry_project", { localUuid, projectId }),
  setEntryBillable: (localUuid: string, billable: boolean) =>
    invoke<void>("set_entry_billable", { localUuid, billable }),
  updateEntryTimes: (localUuid: string, startAt: string, endAt: string) =>
    invoke<void>("update_entry_times", { localUuid, startAt, endAt }),

  listToday: () => invoke<Entry[]>("list_today"),
  listBetween: (from: string, to: string) =>
    invoke<Entry[]>("list_between", { from, to }),

  listProjects: () => invoke<Project[]>("list_projects"),
  refreshProjects: () => invoke<number>("refresh_projects"),
  listOrganizations: () => invoke<OrgChoice[]>("list_organizations"),

  configShow: () => invoke<ConfigEntry[]>("config_show"),
  configSet: (key: string, value: string) =>
    invoke<void>("config_set", { key, value }),
  configTest: () => invoke<string>("config_test"),
  solidtimeUrl: () => invoke<string | null>("solidtime_url"),

  syncNow: () => invoke<number>("sync_now"),
};

export type SolidtimeAuthStatus = {
  mode: "api_token" | "oauth";
  signed_in: boolean;
  scope: string | null;
};

export const oauthSolidtimeStatus = () =>
  invoke<SolidtimeAuthStatus>("oauth_solidtime_status");

export const oauthSolidtimeStart = () =>
  invoke<void>("oauth_solidtime_start");

export const oauthSolidtimeLogout = () =>
  invoke<void>("oauth_solidtime_logout");

export type ConflictInfo = {
  remote_id: string;
  remote_description: string;
  remote_start_at: string;
  local_local_uuid: string;
  local_description: string;
};

export type PullReport = {
  adopted: string | null;
  conflict: ConflictInfo | null;
  inserted: number;
  updated: number;
  deleted: number;
};

export const pullNow = () => invoke<PullReport>("pull_now");

export type ConflictAction = "stop_remote" | "switch" | "dismiss";

export const conflictResolve = (action: ConflictAction, remoteId: string) =>
  invoke<void>("conflict_resolve", {
    args: { action, remote_id: remoteId },
  });

export const calendarApi = {
  listAccounts: () => invoke<CalendarAccount[]>("calendar_list_accounts"),
  oauthStatus: (accountId: string) =>
    invoke<CalendarOAuthStatus>("calendar_oauth_status", { accountId }),
  addGoogle: () => invoke<CalendarAccount>("calendar_add_google"),
  removeAccount: (accountId: string) =>
    invoke<void>("calendar_remove_account", { accountId }),
  listCalendars: (accountId: string) =>
    invoke<CalendarRow[]>("calendar_list_calendars", { accountId }),
  setCalendarIncluded: (calendarId: string, included: boolean) =>
    invoke<void>("calendar_set_calendar_included", { calendarId, included }),
  setDefaultProject: (calendarId: string, projectId: string | null) =>
    invoke<void>("calendar_set_default_project", { calendarId, projectId }),
  refreshAccount: (accountId: string) =>
    invoke<number>("calendar_refresh_account", { accountId }),
  listEventsInRange: (accountId: string, from: string, to: string) =>
    invoke<CalendarEventWithDecision[]>("calendar_list_events_in_range", {
      accountId,
      from,
      to,
    }),
  logEvent: (accountId: string, eventId: string, eventStart: string) =>
    invoke<string>("calendar_log_event", { accountId, eventId, eventStart }),
  ignoreEvent: (accountId: string, eventId: string, eventStart: string) =>
    invoke<void>("calendar_ignore_event", { accountId, eventId, eventStart }),
  revertEvent: (accountId: string, eventId: string, eventStart: string) =>
    invoke<void>("calendar_revert_event", { accountId, eventId, eventStart }),
};
