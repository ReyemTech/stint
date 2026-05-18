-- Phase 1 schema. Calendar tables are deferred to Phase 3.

CREATE TABLE settings (
  key        TEXT PRIMARY KEY,
  value      TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE time_entries (
  local_uuid      TEXT PRIMARY KEY,
  solidtime_id    TEXT UNIQUE,
  description     TEXT NOT NULL DEFAULT '',
  project_id      TEXT,
  task_id         TEXT,
  start_at        TEXT NOT NULL,
  end_at          TEXT,
  billable        INTEGER NOT NULL DEFAULT 0,
  source          TEXT NOT NULL,
  source_event_id TEXT,
  sync_state      TEXT NOT NULL,
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL
);
CREATE INDEX idx_time_entries_start ON time_entries(start_at);
CREATE INDEX idx_time_entries_sync  ON time_entries(sync_state) WHERE sync_state != 'synced';

CREATE TABLE entry_tags (
  local_uuid TEXT NOT NULL REFERENCES time_entries(local_uuid) ON DELETE CASCADE,
  tag_id     TEXT NOT NULL,
  PRIMARY KEY (local_uuid, tag_id)
);

CREATE TABLE running_timer (
  id            INTEGER PRIMARY KEY CHECK (id = 1),
  local_uuid    TEXT NOT NULL REFERENCES time_entries(local_uuid) ON DELETE CASCADE,
  heartbeat_at  TEXT NOT NULL
);

CREATE TABLE projects (
  id         TEXT PRIMARY KEY,
  name       TEXT NOT NULL,
  color      TEXT,
  client_id  TEXT,
  archived   INTEGER NOT NULL DEFAULT 0,
  fetched_at TEXT NOT NULL
);

CREATE TABLE tasks (
  id         TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  name       TEXT NOT NULL,
  done       INTEGER NOT NULL DEFAULT 0,
  fetched_at TEXT NOT NULL
);

CREATE TABLE tags (
  id         TEXT PRIMARY KEY,
  name       TEXT NOT NULL,
  fetched_at TEXT NOT NULL
);

CREATE TABLE sync_queue (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  op          TEXT NOT NULL,
  payload     TEXT NOT NULL,
  attempts    INTEGER NOT NULL DEFAULT 0,
  last_error  TEXT,
  enqueued_at TEXT NOT NULL,
  next_try_at TEXT NOT NULL,
  entry_uuid  TEXT
);
CREATE INDEX idx_sync_queue_next_try ON sync_queue(next_try_at);
