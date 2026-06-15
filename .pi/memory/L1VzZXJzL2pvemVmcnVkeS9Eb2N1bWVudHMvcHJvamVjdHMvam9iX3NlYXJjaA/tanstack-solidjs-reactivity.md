---
type: lesson
tags: [solidjs, tanstack-query, reactivity]
created: 2026-06-15
updated: 2026-06-15
---

Use TanStack Query v5 with SolidJS: set `structuralSharing: false` on every `createQuery` so refetches produce new refs; render query data with `<Show keyed when={data}>`. Plain `<Show>` tracks truthiness only, and TanStack's default mutates objects in place, so stale refs break UI updates.
