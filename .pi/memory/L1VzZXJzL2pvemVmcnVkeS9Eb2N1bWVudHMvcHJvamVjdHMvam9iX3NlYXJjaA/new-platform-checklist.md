---
type: lesson
tags: [jobsearch, platforms, scraping, checklist]
created: 2026-06-15
updated: 2026-06-15
---

For a new job board, start with a CLI-only spike: confirm search cards, pagination, detail selectors, URL encoding, and bot protection. Then wire into `Platform`, `Data`, `cli.rs`, `main.rs`, `display.rs`, and integration tests. When adding a `Platform` variant, also update `db.rs` `test_job()` and any `Platform` match in `display.rs`.
