---
type: lesson
tags: [jobsearch, budget, parsing, regex]
created: 2026-06-13
updated: 2026-06-13
---

Budget parsing prefers a single regex that handles separators with optional whitespace and repeated currency symbols. Keep `parse_range` as regex over chained `split_once` calls — easier to extend for formats like `$130,530 to 221,920 USD`, `$50-$100/hr`, `7 069 – 9 426 EUR`.
