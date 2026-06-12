---
type: context
tags: [jobsearch, efinancialcareers, applications-sync, todo]
created: 2026-06-12
updated: 2026-06-12
---

eFinancialCareers applications sync TODO: scrape `https://www.efinancialcareers.com/myefc/my-jobs` after implementing basic job search. Look for `.job-card-blue-text` links to identify applied jobs. Follow same pattern as Upwork/NoFluffJobs: fetch applications list, fetch job detail by ID/slug, upsert job, then mark applied. Skip items where detail is unavailable.
