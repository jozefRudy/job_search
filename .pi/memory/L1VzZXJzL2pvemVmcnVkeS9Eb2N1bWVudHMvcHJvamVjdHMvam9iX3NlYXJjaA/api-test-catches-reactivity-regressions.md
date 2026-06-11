---
type: lesson
tags: [solidjs, tanstack-query, orval, testing, reactivity]
created: 2026-06-11
updated: 2026-06-11
---

API-level tests with mocked fetch are essential for catching silent reactivity regressions in SolidJS + TanStack Query + orval setups. What breaks silently:

1. **Reactive queries stop refetching** — orval-generated `useListJobs` captures params as static closure. Switching to it = stale data forever, no error.
2. **Cache invalidation misses** — wrong query keys (`["jobs"]` vs `getListJobsQueryKey()`) = mutation succeeds but UI never updates.
3. **Unwrapped response types** — forgetting `.data` or `unwrap()` = `query.data` becomes `{}`, no compile error.

Tests in `frontend/src/api.test.tsx` verify all three: reactive refetch, cache invalidation, correct key usage. Run `pnpm test run` after any API change. Prevents deployment of broken reactivity.
