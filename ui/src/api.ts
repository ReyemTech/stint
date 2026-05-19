import { invoke } from "@tauri-apps/api/core";
import type { ConfigEntry, Entry, OrgChoice, Project, RunningTimer } from "./types";

export const api = {
  getRunningTimer: () => invoke<RunningTimer | null>("get_running_timer"),
  startTimer: (
    description: string,
    projectId?: string | null,
    taskId?: string | null,
    billable = false,
  ) =>
    invoke<string>("start_timer", {
      args: {
        description,
        project_id: projectId ?? null,
        task_id: taskId ?? null,
        billable,
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

  syncNow: () => invoke<number>("sync_now"),
};
