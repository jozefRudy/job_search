-- Generated column for DB-level sorting of Upwork-specific fields.
-- VIRTUAL = computed on read, no storage bloat. No backfill needed.
-- SQLite lacks jsonb; JSON lives in TEXT with json_extract() for querying.

ALTER TABLE jobs ADD COLUMN upwork_last_viewed_at TEXT GENERATED ALWAYS AS (
  CASE platform
    WHEN 'upwork' THEN json_extract(raw, '$.detail.last_viewed')
    ELSE NULL
  END
) VIRTUAL;
