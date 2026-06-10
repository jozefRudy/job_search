---
type: lesson
tags: [solidjs, typescript, frontend, design]
created: 2026-06-10
updated: 2026-06-10
---

In SolidJS projects, presentation-layer helpers (`fmtRelative`, `ratingEmoji`, `ratingClass`, etc.) should accept `null | undefined` in their signatures and return sensible defaults. Two sources of nullable values:

1. **API data via Orval** — optional fields are typed `string | null` (from OpenAPI `type: ["string", "null"]`)
2. **TanStack Query `data`** — starts `undefined` while loading, may be stale/undefined between fetches

Business logic functions should stay strict — fail fast on null/undefined.

Example:
```ts
// Good — handles both API nulls and query undefined gracefully
export function fmtRelative(dtStr: string | null | undefined): string {
  if (!dtStr) return "";
  // ...
}

// Bad — forces compensating `?? null` at every call site
export function fmtRelative(dtStr: string | null): string { ... }
// Usage: fmtRelative(j.applied_at ?? null) // noise everywhere
```
