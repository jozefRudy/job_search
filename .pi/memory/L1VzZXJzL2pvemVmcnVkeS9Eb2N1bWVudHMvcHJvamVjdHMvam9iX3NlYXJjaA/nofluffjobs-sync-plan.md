---
type: context
created: 2026-06-12
updated: 2026-06-12
---

NoFluffJobs applications sync uses `/api/candidates/my-applications` with HMAC auth from `nfj_session`/`nfj_salt` cookies, paginated. Job detail is fetched via existing public `/api/posting/{postingId}`. Deduplicate by `postingId` keeping latest; skip if job already in DB; set `applied_at` from earliest `statusHistory` status `applied`, fallback to `appliedDate`.
