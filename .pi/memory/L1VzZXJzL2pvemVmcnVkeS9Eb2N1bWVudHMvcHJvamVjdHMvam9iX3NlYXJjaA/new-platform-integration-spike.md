---
type: lesson
tags: [jobsearch, scraping, spike]
created: 2026-06-12
updated: 2026-06-13
---

When integrating a new job board, scrape enough detail to make the listing useful in CLI/server without over-engineering. For eFinancialCareers, we confirmed: use Brave browser (bot protection blocks curl), search cards are `<efc-job-card>`, pagination is "Show more", detail description lives in `<efc-job-description>`, and URL encoding is non-standard (`+` for spaces, `%7C` for pipes). Start with a CLI-only spike, then wire into `Platform`, `Data`, `cli.rs`, `main.rs`, `display.rs`, and integration tests.
