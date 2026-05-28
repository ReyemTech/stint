export interface EntryDTO {
  local_uuid: string;
  solidtime_id: string | null;
  description: string;
  project_id: string | null;
  task_id: string | null;
  billable: boolean;
  start_at: string;
  end_at: string | null;
  source: string;
}

export interface ProjectDTO {
  solidtime_id: string;
  name: string;
  color: string | null;
  client_id: string | null;
  archived: boolean;
}

export interface TaskDTO {
  solidtime_id: string;
  project_id: string;
  name: string;
  done: boolean;
}
