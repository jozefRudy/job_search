---
type: lesson
tags: [frontend, orval, tanstack, solidjs]
created: 2026-06-10
updated: 2026-06-10
---

Orval `client: 'solid-query'` was broken for TanStack Query v5 but is now FIXED in v8.16.0 (PR [#3369](https://github.com/orval-labs/orval/pull/3369)). The generated code uses `MutationOptions`/`UseQueryOptions` from `@tanstack/solid-query` (not removed `SolidMutationOptions`), and `useQuery`/`useMutation` are valid exports in v5 (aliased as `createQuery`/`createMutation`).

**Current setup:** `client: 'solid-query'` in `orval.config.js`. `api.ts` uses generated `get*QueryKey` + `listJobs`/`getJob`/`deleteJob`/`rateJob` functions with manual `createQuery`/`createMutation` wrappers for response unwrapping and app callbacks.

**Why manual wrappers remain:** Generated `get*QueryOptions` spreads into `createQuery` cause TypeScript conflicts because `queryFn` return type (raw response) differs from unwrapped data type. Manual `queryFn` with error unwrapping is cleaner.
