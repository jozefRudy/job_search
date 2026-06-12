---
type: lesson
tags: [jobsearch, scraping, spike]
created: 2026-06-12
updated: 2026-06-12
---

When integrating a new job board, scrape enough detail to make the listing useful in CLI/server without over-engineering. For eFinancialCareers, start with a small CLI-only spike: fetch search results via curl, identify detail endpoint and required headers/cookies, then build a minimal scraper that produces the same `Job` shape as existing platforms. Do not add persistence changes until basic fetch works end-to-end.
