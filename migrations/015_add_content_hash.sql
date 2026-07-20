ALTER TABLE jobs ADD COLUMN content_hash TEXT;
CREATE INDEX idx_jobs_content_hash ON jobs(content_hash);
