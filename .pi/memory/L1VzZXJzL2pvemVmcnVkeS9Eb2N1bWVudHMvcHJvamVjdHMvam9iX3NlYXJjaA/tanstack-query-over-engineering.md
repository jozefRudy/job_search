---
type: lesson
tags: [tanstack-query, solidjs, orval, frontend]
created: 2026-06-11
updated: 2026-06-11
---

Don't over-engineer TanStack Query cache invalidation. User prefers short, maintainable code. Simple `onSuccess` invalidation with correct query keys is sufficient for most cases. Optimistic updates add significant complexity for marginal UX gain. When user pushes back on code length, listen — they likely want the simplest working solution, not the most robust one.
