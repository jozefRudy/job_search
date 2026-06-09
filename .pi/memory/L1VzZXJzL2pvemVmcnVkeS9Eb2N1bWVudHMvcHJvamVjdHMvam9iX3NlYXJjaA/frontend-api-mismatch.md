---
type: lesson
tags: [frontend, api, rust, solidjs]
created: 2026-06-09
updated: 2026-06-09
---

Frontend `api.ts` must unwrap server response shape. Server `list_jobs` returns `{ jobs: Vec<Job> }` but frontend `listJobs` initially returned raw `res.json()` causing `TypeError: object is not iterable`. Always check API response wrapper matches frontend expectation.
