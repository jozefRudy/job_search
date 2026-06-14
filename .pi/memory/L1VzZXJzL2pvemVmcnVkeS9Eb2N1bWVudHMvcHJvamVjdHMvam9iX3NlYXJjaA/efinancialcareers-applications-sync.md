---
type: lesson
tags: [efinancialcareers, sync, api, job-activities, batch]
created: 2026-06-14
updated: 2026-06-14
---

eFinancialCareers My Jobs popup descriptions come from `https://job.efinancialcareers.com/api/v1/jobs/batch?job_ids=<internal_job_id>&response_properties=title,summary,description`. The internal `job_id` (e.g. `WzPsA2Wb22h3NiEM`) is in the `job-activities` API response. Standalone detail pages 404/410 for expired jobs, but this batch endpoint still returns description.

Rob sync flow:
1. Fetch `job-activities?jobseekerId=...` from page using `myEfcCookieAuth` JWT.
2. Filter `status == APPLIED`; keep `job_activity_id`, internal `job_id`, title, URL, salary, company, location, applied_at.
3. For missing jobs, call batch endpoint with internal `job_id` to get description.
4. Upsert job and mark applied.

Avoid clicking modals — use the batch API directly.
