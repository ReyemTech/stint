-- Phase 3b: calendar tables. Matches spec §3 ("Calendar").

CREATE TABLE calendar_accounts (
  id           TEXT PRIMARY KEY,
  provider     TEXT NOT NULL,                  -- 'google' (Phase 3c/d add 'microsoft', 'caldav')
  display_name TEXT NOT NULL,
  identifier   TEXT NOT NULL,                  -- email for OAuth providers
  caldav_url   TEXT,                           -- nullable; populated only for CalDAV (Phase 3d)
  enabled      INTEGER NOT NULL DEFAULT 1,
  created_at   TEXT NOT NULL
);

CREATE TABLE calendars (
  id         TEXT PRIMARY KEY,                 -- provider-native calendar id
  account_id TEXT NOT NULL REFERENCES calendar_accounts(id) ON DELETE CASCADE,
  name       TEXT NOT NULL,
  color      TEXT,
  included   INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_calendars_account ON calendars(account_id);

CREATE TABLE calendar_events (
  id              TEXT NOT NULL,               -- provider event id
  account_id      TEXT NOT NULL REFERENCES calendar_accounts(id) ON DELETE CASCADE,
  calendar_id     TEXT NOT NULL REFERENCES calendars(id) ON DELETE CASCADE,
  title           TEXT NOT NULL,
  start_at        TEXT NOT NULL,
  end_at          TEXT NOT NULL,
  is_all_day      INTEGER NOT NULL DEFAULT 0,
  attendee_status TEXT,                        -- 'accepted' | 'declined' | 'tentative' | NULL
  recurring_root  TEXT,                        -- provider's recurringEventId for expanded instances
  fetched_at      TEXT NOT NULL,
  PRIMARY KEY (account_id, id, start_at)
);
CREATE INDEX idx_calendar_events_start ON calendar_events(start_at);
CREATE INDEX idx_calendar_events_calendar_start ON calendar_events(calendar_id, start_at);

CREATE TABLE event_decisions (
  account_id        TEXT NOT NULL,
  event_id          TEXT NOT NULL,
  event_start       TEXT NOT NULL,
  decision          TEXT NOT NULL,             -- 'ignored' | 'logged_manual' | 'logged_auto'
  linked_local_uuid TEXT REFERENCES time_entries(local_uuid) ON DELETE SET NULL,
  decided_at        TEXT NOT NULL,
  PRIMARY KEY (account_id, event_id, event_start)
);
