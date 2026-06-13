---
type: lesson
tags: [jobsearch, platform, refactoring, checklist]
created: 2026-06-13
updated: 2026-06-13
---

When a new `Platform` variant is added, also update `db.rs` test helper `test_job()` and any `match` on `Platform` in `display.rs`. Otherwise compile errors surface only in tests or UI rendering.
