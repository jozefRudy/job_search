---
type: lesson
tags: [solidjs, frontend, url-params, zod, filters]
created: 2026-06-15
updated: 2026-06-15
---

URL query params are source of truth for list/filter state in SolidJS. Use `useSearchParams` + Zod `.nullable().catch(null)`; `null` means "no filter", router strips it from URL, missing param falls back to `null`. Render select value as `filter() ?? "all"` and update with `{ filter: value, page: "" }`. Strip `null` before API call (e.g. `pickBy(isNotNil)`). Use `navigate(-1)` for back buttons to preserve params/history. Avoid sentinel strings, manual `String()` round-trips, and duplicating state in signals/services/stores.
