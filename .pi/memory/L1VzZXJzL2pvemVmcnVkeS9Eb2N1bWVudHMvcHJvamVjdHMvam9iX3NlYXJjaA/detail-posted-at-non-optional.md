---
type: lesson
tags: [refactoring, data-model, rust, option-types]
created: 2026-06-16
updated: 2026-06-16
---

When a scraped field like `posted_at` is the source of truth for a non-optional DB column (`jobs.created_at`), model it as non-optional and push the fallback to `Utc::now()` into the platform boundary parsers. Keeps downstream code free of `unwrap_or_else(Utc::now)` duplication and removes ambiguity about whether a missing value is meaningful.
