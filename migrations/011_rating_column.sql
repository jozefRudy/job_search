-- Replace nullable liked with explicit rating enum.
-- Existing NULL rows become 'neutral'.

ALTER TABLE jobs ADD COLUMN rating TEXT NOT NULL DEFAULT 'neutral';
UPDATE jobs SET rating = CASE
    WHEN liked = 1 THEN 'liked'
    WHEN liked = 0 THEN 'disliked'
    ELSE 'neutral'
END;

CREATE TABLE jobs_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    platform TEXT NOT NULL,
    external_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    url TEXT NOT NULL,
    budget TEXT,
    tags TEXT NOT NULL,
    raw TEXT NOT NULL,
    upwork_last_viewed_at TEXT GENERATED ALWAYS AS (
      CASE platform
        WHEN 'upwork' THEN json_extract(raw, '$.detail.last_viewed')
        ELSE NULL
      END
    ) VIRTUAL,
    company TEXT GENERATED ALWAYS AS (
      CASE platform
        WHEN 'upwork' THEN NULL
        WHEN 'nofluffjobs' THEN json_extract(raw, '$.detail.company')
        WHEN 'efinancialcareers' THEN json_extract(raw, '$.detail.company')
        WHEN 'hackernews' THEN json_extract(raw, '$.detail.company')
        ELSE NULL
      END
    ) VIRTUAL,
    rating TEXT NOT NULL DEFAULT 'neutral',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    remote BOOL NOT NULL DEFAULT 1,
    is_english BOOL NOT NULL DEFAULT 1,
    UNIQUE(platform, external_id)
);

INSERT INTO jobs_new (
    id, platform, external_id, title, description, url, budget, tags, raw,
    rating, created_at, updated_at, remote, is_english
)
SELECT
    id, platform, external_id, title, description, url, budget, tags, raw,
    rating, created_at, updated_at, remote, is_english
FROM jobs;

DROP TABLE jobs;
ALTER TABLE jobs_new RENAME TO jobs;
