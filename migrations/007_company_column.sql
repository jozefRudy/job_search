-- Generated column for DB-level access to the company/client name.
-- VIRTUAL = computed on read, no storage bloat. No backfill needed.

ALTER TABLE jobs ADD COLUMN company TEXT GENERATED ALWAYS AS (
  CASE platform
    WHEN 'upwork' THEN NULL
    WHEN 'nofluffjobs' THEN json_extract(raw, '$.detail.company')
    WHEN 'efinancialcareers' THEN json_extract(raw, '$.detail.company')
    WHEN 'hackernews' THEN json_extract(raw, '$.detail.company')
    ELSE NULL
  END
) VIRTUAL;
