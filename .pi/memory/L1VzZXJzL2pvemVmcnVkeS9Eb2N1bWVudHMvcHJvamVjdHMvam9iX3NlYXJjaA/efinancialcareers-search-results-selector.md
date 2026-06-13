---
type: lesson
tags: [scraping, efinancialcareers, dom]
created: 2026-06-13
updated: 2026-06-13
---

On eFinancialCareers search pages, matched results are rendered inside `<efc-job-search-results>`; suggested/extra listings live in separate components (`<efc-recommended-jobs>`, `<efc-matching-jobs>`, `<efc-popular-jobs>`). Scope card selectors to `efc-job-search-results efc-job-card` to avoid processing unrelated jobs.
