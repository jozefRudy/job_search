-- Old records store raw relative-time strings (e.g. "last week", "3 days ago")
-- in detail.last_viewed. Current schema stores parsed ISO-8601 datetimes.
-- Generated column upwork_last_viewed_at extracts this path directly, so
-- raw strings sort to the top lexicographically before ISO datetimes.
-- Backfill: convert non-ISO values to JSON null so they become SQL NULL
-- and sort correctly with NULLS LAST.

UPDATE jobs
SET raw = json_set(raw, '$.detail.last_viewed', NULL)
WHERE platform = 'upwork'
  AND json_extract(raw, '$.detail.last_viewed') IS NOT NULL
  AND json_extract(raw, '$.detail.last_viewed') NOT LIKE '____-__-__T%';
