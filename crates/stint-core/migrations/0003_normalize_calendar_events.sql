-- Drop calendar_events rows whose start_at is in RFC 3339 offset form
-- (e.g. "2026-05-20T14:15:00-04:00") rather than UTC Z form
-- (e.g. "2026-05-20T18:15:00Z"). These were created before the Google
-- DTO started normalizing to Z form — the composite PK includes start_at,
-- so the renormalized refresh INSERTed sibling rows instead of updating.
--
-- The next calendar refresh (within 15 min of app start) re-fetches and
-- re-inserts these in Z form via the normalized DTO path. We keep all-day
-- events (YYYY-MM-DD, no time component) since those never had this issue.

DELETE FROM calendar_events
WHERE start_at NOT LIKE '%Z'
  AND start_at NOT GLOB '????-??-??';
