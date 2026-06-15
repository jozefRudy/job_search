---
type: lesson
tags: [efinancialcareers, scraping, browser, sync, api]
created: 2026-06-15
updated: 2026-06-15
---

Use Brave/CDP for search (bot protection blocks curl). Matched results live in `<efc-job-search-results>`; zero-results marker is `efc-empty-job-search-results-wrapper`; pagination is "Show more". Scope card selectors to `efc-job-search-results efc-job-card`. Filter URL uses `|` (%7C) for OR and literal `+` for spaces. Detail pages may 404 for expired jobs; application sync uses `job-activities` API for internal `job_id`s, then batch API `job.efinancialcareers.com/api/v1/jobs/batch?job_ids=...` for descriptions. Progress shows processed count only; scraped totals are unreliable.
