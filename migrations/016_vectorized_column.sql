ALTER TABLE jobs ADD COLUMN vectorized BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX IF NOT EXISTS idx_jobs_vectorized ON jobs(vectorized);
