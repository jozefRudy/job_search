---
type: lesson
tags: [models, db, conventions, scraping]
created: 2026-06-16
updated: 2026-06-16
---

In this project, `Job::created_at` and the DB `jobs.created_at` column store the parsed job-posting time from the platform (e.g., `posted_at_text` from Upwork detail/card), not the row insertion time. `Job::posted_at` exists as an ephemeral `Option<DateTime<Utc>>` and is currently not persisted. Before making assumptions about field semantics, read the platform mappers and DB upsert code rather than guessing from field names.
