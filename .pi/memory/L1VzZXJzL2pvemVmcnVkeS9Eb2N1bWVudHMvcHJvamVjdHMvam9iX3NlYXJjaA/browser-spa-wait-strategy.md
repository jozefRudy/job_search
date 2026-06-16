---
type: lesson
tags: [browser, chromiumoxide, spa]
created: 2026-06-15
updated: 2026-06-15
---

For SPA-rendered job boards (Upwork, NoFluffJobs, eFinancialCareers), prefer element polling over `wait_for_navigation`. `goto` already awaits navigation lifecycle; SPA content renders later, so `wait_for_element` is the reliable signal.
