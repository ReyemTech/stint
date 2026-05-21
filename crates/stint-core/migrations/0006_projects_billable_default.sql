-- Solidtime's project schema exposes is_billable, which is the default
-- billable state new time entries against that project inherit. Cache it
-- locally so calendar_log_event can apply it without a network round trip.
ALTER TABLE projects ADD COLUMN billable_default INTEGER NOT NULL DEFAULT 0;
