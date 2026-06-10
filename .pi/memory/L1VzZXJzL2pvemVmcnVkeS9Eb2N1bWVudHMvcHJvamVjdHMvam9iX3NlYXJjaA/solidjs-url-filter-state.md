---
type: lesson
tags: [solidjs, frontend, url-params, zod]
created: 2026-06-10
updated: 2026-06-10
---

For persistent filter state in SolidJS: use `@solidjs/router`'s `useSearchParams`, not local signals. Back button restores state, links are shareable. Use `z.union([z.literal(...)]).catch(default)` for parsing/validation instead of manual `if` chains. Use `"all"` as explicit sentinel for "no filter" — `@solidjs/router` strips `null`/`undefined`/`""` from URL. Prefer `Platform | "all"` over `Platform | null` everywhere; only convert to `undefined` at API boundary with `pickBy`. Use `setSearchParams(..., { replace: true })` to avoid history pollution.
