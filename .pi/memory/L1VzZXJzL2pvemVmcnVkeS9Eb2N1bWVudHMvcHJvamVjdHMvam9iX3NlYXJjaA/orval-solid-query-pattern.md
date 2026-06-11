---
type: context
tags: [orval, solidjs, tanstack-query, frontend]
created: 2026-06-11
updated: 2026-06-11
---

With orval `client: 'solid-query'`, use generated hooks for mutations but custom `createQuery` wrappers for queries. Orval-generated query hooks capture params as static closure values — incompatible with SolidJS fine-grained reactivity. **Upstream bug: orval#3195** ("solid-query: make queries reactive"), open with no fix.

Pattern for queries: use orval's generated `get*QueryKey` + `listJobs`/`getJob` fetch functions inside `createQuery(() => ({ queryKey: ..., queryFn: async () => { const res = await ...; if (res.status !== 200) throw ...; return res.data; }, structuralSharing: false }))`. The status check is required because TanStack Query treats resolved promises as success — a 500 response would cache empty data without it.

Use `unwrap()` helper typed as `Promise<{ data: T; status: 200 } | { status: number }>` — error branch omits `data` entirely, no `void`/`undefined` friction with orval's generated types.

Pattern for mutations: use orval-generated hooks directly (`useRateJobGen`, `useDeleteJobGen`) with `mutation` options for cache invalidation via `qc.invalidateQueries({ queryKey: getListJobsQueryKey() })`. No need to recreate `createMutation`.

API-level tests in `frontend/src/api.test.tsx` verify:
- Reactive params trigger refetch (useListJobs)
- Reactive id trigger refetch (useGetJob)
- Rate mutation invalidates queries on success (useRateJob)
- Delete mutation invalidates queries on success (useDeleteJob)

Mock `@solidjs/router` in tests when hooks use `useNavigate()`. These tests prove the API/cache layer is correct; component-level reactivity builds on this foundation.

Always run `pnpm test run` after API changes to prevent regressions.
