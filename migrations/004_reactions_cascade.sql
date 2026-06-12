PRAGMA foreign_keys=off;

CREATE TABLE IF NOT EXISTS reactions_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id INTEGER NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    note TEXT,
    applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UNIQUE(job_id)
);

INSERT INTO reactions_new (id, job_id, note, applied_at)
SELECT id, job_id, note, applied_at FROM reactions;

DROP TABLE reactions;
ALTER TABLE reactions_new RENAME TO reactions;

PRAGMA foreign_keys=on;
