---
type: lesson
tags: [efinancialcareers, sync, batch-api, job-activities]
created: 2026-06-14
updated: 2026-06-14
---

eFinancialCareers application sync should use the batch API (`job.efinancialcareers.com/api/v1/jobs/batch?job_ids=...&response_properties=title,summary,description`) instead of clicking My Jobs modals. Modal clicking is brittle and slow; batch endpoint returns description even for expired jobs where detail pages 404.
