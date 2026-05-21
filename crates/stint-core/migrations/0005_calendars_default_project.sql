-- Phase 3d: per-calendar default project for "Log this" prefill.
-- No FK to projects(id) because Solidtime projects can be deleted on the
-- server; we never want a delete there to fail a constraint here. The
-- calendar_log_event path silently treats a stale id as "no project"
-- (Solidtime returns 422 only on member_id, not on project_id mismatch).

ALTER TABLE calendars ADD COLUMN default_project_id TEXT;
