---
type: lesson
tags: [solidjs, zod, url-params, filters]
created: 2026-06-10
updated: 2026-06-10
---

For persistent filter state in SolidJS: use `useSearchParams` + Zod `.nullable().catch(null)`. `null` means "no filter" in data model; HTML `<option value="all">` maps to `null` via `onChange`. Router strips `null` from URL, so on read back missing param → zod catch → `null`. Select shows "all" via `value={platform() ?? "all"}`.

**Critical:** "Back" from detail to list must use `navigate(-1)` (browser back), NOT `navigate("/")`. The latter pushes a new navigation to `/` with no query params, destroying filter state. `navigate(-1)` preserves URL params and history stack.
