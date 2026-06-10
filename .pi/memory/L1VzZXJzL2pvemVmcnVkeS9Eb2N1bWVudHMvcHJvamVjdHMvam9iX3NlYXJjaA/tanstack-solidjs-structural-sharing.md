---
type: lesson
tags: [frontend, tanstack, solidjs, reactivity]
created: 2026-06-10
updated: 2026-06-10
---

When using TanStack Query v5 with SolidJS, always set `structuralSharing: false` on queries. TanStack's default mutates objects in place, keeping same references. SolidJS compares by reference (`===`), so `<Show when={data}>` and derived reactivity won't trigger updates.

Additionally, use `<Show keyed when={data}>` (not plain `<Show>`) when rendering query data. Solid's `<Show>` tracks truthiness only — changing from truthy object A to truthy object B does NOT recreate children. `keyed` compares by `===` and forces recreation when the reference changes.

Example:
```tsx
// Wrong — UI stays stale after mutation refetch
<Show when={query.data}>
  {(j) => <Detail job={j()} />}
</Show>

// Right — recreates Detail with fresh data
<Show when={query.data} keyed>
  {(j) => <Detail job={j} />}
</Show>
```
