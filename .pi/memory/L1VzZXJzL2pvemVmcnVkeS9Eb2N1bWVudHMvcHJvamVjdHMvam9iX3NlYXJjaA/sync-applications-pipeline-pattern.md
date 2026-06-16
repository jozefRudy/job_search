---
type: lesson
tags: [sync-applications, platforms, pattern]
created: 2026-06-16
updated: 2026-06-16
---

All three providers share the same `sync_applications` pipeline: collect platform-specific application items; for each item call `db.find_job_id()` and fetch+upsert detail only when missing; then check stored `applied_at` to decide `existing` vs `new`, and call `db.set_applied()`. Keep provider-specific auth/pagination inside the scraper, but emit counts through shared `FetchState`.
