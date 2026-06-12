---
type: lesson
tags: [rust, serde, types, scraping, boundary]
created: 2026-06-12
updated: 2026-06-12
---

When scraping external APIs, do shape-normalization in explicit `Raw*` structs plus `TryFrom` at the boundary, not by manual mutation after parsing. `TryFrom` lets the caller skip malformed items instead of crashing the whole sync. Keep `From` only for infallible internal conversions. Both Upwork and NoFluffJobs now use `TryFrom<Raw*>` for scraped detail/application data.
