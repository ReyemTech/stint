-- Phase 3d: client cache. Mirrors the projects/tasks/tags pattern.

CREATE TABLE clients (
  id         TEXT PRIMARY KEY,
  name       TEXT NOT NULL,
  archived   INTEGER NOT NULL DEFAULT 0,
  fetched_at TEXT NOT NULL
);
