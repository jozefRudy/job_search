---
type: context
tags: [frontend, tanstack, solidjs, reactivity]
created: 2026-06-10
updated: 2026-06-10
---

Idiomatic SolidJS + TanStack Query v5 pattern for reactive data fetching:

1. Set `structuralSharing: false` on all `createQuery` calls — TanStack's default mutates objects in place, keeping same refs. SolidJS compares by `===`, so stale refs = no UI updates.

2. Use `<Show keyed when={data}>` (never plain `<Show>`) when rendering query results. Solid's `<Show>` tracks truthiness only — truthy A → truthy B with same ref does NOT recreate. `keyed` compares by `===` and forces recreation when ref changes.

3. Both required together. `structuralSharing: false` gives new refs on refetch; `keyed` makes Solid actually recreate children with new data.

Example:
```tsx
// api.ts
export function useJob(id: () => number) {
  return createQuery<Job>(() => ({
    queryKey: ["job", id()],
    queryFn: async () => { ... },
    structuralSharing: false, // Required for Solid
  }));
}

// Component.tsx
const jobQuery = useJob(id);

<Show when={jobQuery.data} keyed fallback={<Skeleton />}>
  {(j) => <Detail job={j} />}
</Show>
```
