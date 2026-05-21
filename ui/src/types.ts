export type RunningTimer = {
  local_uuid: string;
  description: string;
  start_at: string;
  project_id: string | null;
  billable: boolean;
};

export type Entry = {
  local_uuid: string;
  solidtime_id: string | null;
  description: string;
  project_id: string | null;
  task_id: string | null;
  start_at: string;
  end_at: string | null;
  billable: boolean;
  sync_state: "synced" | "dirty" | "pending_create" | "pending_delete";
  source: string;
};

export type Project = {
  id: string;
  name: string;
  color: string | null;
  client_id: string | null;
  client_name: string | null;
  archived: number;
};

export type OrgChoice = {
  id: string;
  member_id: string;
  name: string;
};

export type ConfigEntry = {
  key: string;
  value: string | null;
  is_secret: boolean;
  present: boolean;
};

export type AppError = {
  kind: string;
  message: string;
};

export type ProviderKind = "google";

export type CalendarAccount = {
  id: string;
  provider: ProviderKind;
  display_name: string;
  identifier: string;
  caldav_url: string | null;
  enabled: boolean;
  created_at: string;
};

export type CalendarRow = {
  id: string;
  account_id: string;
  name: string;
  color: string | null;
  included: boolean;
};

export type CalendarEventDecision = "ignored" | "logged_manual" | "logged_auto";

export type CalendarEventWithDecision = {
  id: string;
  account_id: string;
  calendar_id: string;
  title: string;
  start_at: string;
  end_at: string;
  is_all_day: boolean;
  attendee_status: "accepted" | "declined" | "tentative" | null;
  recurring_root: string | null;
  fetched_at: string;
  decision: CalendarEventDecision | null;
  linked_local_uuid: string | null;
};

export type CalendarOAuthStatus = {
  signed_in: boolean;
  scope: string | null;
};
