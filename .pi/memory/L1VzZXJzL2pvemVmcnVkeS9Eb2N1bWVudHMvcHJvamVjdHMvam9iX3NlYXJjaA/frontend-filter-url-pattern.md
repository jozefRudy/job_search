---
type: lesson
tags: [frontend, solidjs, pattern, filters]
created: 2026-06-15
updated: 2026-06-15
---

Frontend optional filter dropdowns should follow a single pattern: parse with `zodSchema.nullable().catch(null)`, render value as `filter() ?? "all"`, and update with `{ filter: value, page: "" }` where `value: T | null`. Solid router omits `null` from URL; `pickBy(isNotNil)` strips it before API call. Avoid sentinel strings like `""` or manual `String()` round-trips.
