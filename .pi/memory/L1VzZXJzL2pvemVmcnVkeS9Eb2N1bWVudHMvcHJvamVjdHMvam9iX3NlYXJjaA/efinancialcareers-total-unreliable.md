---
type: lesson
tags: [scraping, efinancialcareers, progress-ui]
created: 2026-06-13
updated: 2026-06-13
---

eFinancialCareers exposes a `transferredData()` script and a visible heading count, but both can be stale or inconsistent with actual loaded cards. For this platform, progress should not rely on a scraped total; show processed count only and stop when pagination/load-more returns false.
