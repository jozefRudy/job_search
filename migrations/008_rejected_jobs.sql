CREATE TABLE IF NOT EXISTS rejected_jobs (
    platform TEXT NOT NULL,
    external_id TEXT NOT NULL,
    reason TEXT NOT NULL,
    rejected_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    PRIMARY KEY (platform, external_id)
);
