CREATE INDEX IF NOT EXISTS idx_jobs_created_at ON jobs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_jobs_upwork_last_viewed ON jobs(upwork_last_viewed_at DESC);
CREATE INDEX IF NOT EXISTS idx_reactions_applied_at ON reactions(applied_at DESC);
