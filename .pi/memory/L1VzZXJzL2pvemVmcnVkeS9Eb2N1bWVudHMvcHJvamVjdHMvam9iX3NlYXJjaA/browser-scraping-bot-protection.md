---
type: lesson
tags: [jobsearch, browser, scraping, bot-protection]
created: 2026-06-13
updated: 2026-06-13
---

For browser-driven scrapers behind bot protection (eFinancialCareers), reuse the existing Brave/CDP session instead of HTTP API. Search cards and detail pages are accessible via authenticated browser context; direct curl hits the WAF captcha.
