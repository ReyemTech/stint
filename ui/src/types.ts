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
