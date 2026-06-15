---
type: lesson
tags: [nofluffjobs, sync, auth, api]
created: 2026-06-15
updated: 2026-06-15
---

NoFluffJobs applications sync uses `/api/candidates/my-applications`, paginated, with HMAC auth from cookies. `nfj_token=<session>:<secret>` replaced the old `nfj_salt`; use the secret part as HMAC key. Job detail comes from public `/api/posting/{postingId}`. Deduplicate by `postingId` keeping latest, skip existing DB jobs, set `applied_at` from earliest `statusHistory` status `applied` (fallback `appliedDate`). Avoid `set_currency_cookie` during sync because CDP cookie manipulation can clobber session/auth cookies.
