---
type: context
tags: [fetch-state, db, optimization]
created: 2026-06-15
updated: 2026-06-15
---

In fetch loops, skip detail fetch for existing jobs via `db.find_job_id()` before calling platform-specific detail endpoint. Existing jobs only cost card scrape + DB lookup. Count them as `existing` in shared `FetchState`.
