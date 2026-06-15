---
type: lesson
tags: [rust, budget, parsing, regex]
created: 2026-06-15
updated: 2026-06-15
---

Centralized `Budget` enum in `src/models.rs` has `Range`/`Single` variants. `Budget::parse(s, default_period)` is the only constructor; each platform normalizes its raw budget string and picks a period (`mo`, `year`, `hr`, `None`). Parsing uses one regex handling separators and repeated currency symbols; on failure keep the original string.
