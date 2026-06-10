---
type: lesson
tags: [solidjs, zod, url-params, filters]
created: 2026-06-10
updated: 2026-06-10
---

Clean pattern for filter state persisted in URL with SolidJS: use `useSearchParams` + Zod `.nullable().catch(null)`. `null` means "no filter" in data model; HTML `<option value="all">` maps to `null` via `onChange`. Router strips `null` from URL, so on read back missing param → zod catch → `null`. Select shows "all" via `value={platform() ?? "all"}`. No sentinel string in the type system — `null` is single source of truth.
