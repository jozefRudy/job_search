---
type: lesson
tags: [jobsearch, upwork, budget, scraping]
created: 2026-06-15
updated: 2026-06-15
---

Upwork detail pages may hide hourly budgets client-side. The server still emits `hourlyBudgetType: "NOT_PROVIDED"` in hydrated state. When DOM budget is empty and that marker is present, store "Budget hidden" in `exact_budget` (same pattern as unparsable budget strings on other platforms). Fixed in `src/platforms/upwork/fetch_job_detail.js`.
