-- Include LinkedIn in the generated company column.
ALTER TABLE jobs DROP COLUMN company;
ALTER TABLE jobs ADD COLUMN company TEXT GENERATED ALWAYS AS (
  CASE platform
    WHEN 'upwork' THEN NULL
    WHEN 'nofluffjobs' THEN json_extract(raw, '$.detail.company')
    WHEN 'efinancialcareers' THEN json_extract(raw, '$.detail.company')
    WHEN 'hackernews' THEN json_extract(raw, '$.detail.company')
    WHEN 'linkedin' THEN json_extract(raw, '$.detail.company')
    ELSE NULL
  END
) VIRTUAL;
