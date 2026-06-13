---
type: lesson
tags: [rust, budget, refactoring]
created: 2026-06-13
updated: 2026-06-13
---

Budget model is a central enum (`Range`/`Single`) in `src/models.rs` with `Display` handling single-value formatting. `Budget::parse(s, default_period)` is the only constructor; no `from_bounds` helpers. Each platform normalizes its raw budget string into a currency/amount string, picks a default period (nofluff `mo`, efinancialcareers `year`, Upwork `hr` for hourly/`None` for fixed), then calls `Budget::parse`. If parsing fails, platform keeps the original string.
