---
type: lesson
tags: [frontend, orval, tanstack, solidjs]
created: 2026-06-10
updated: 2026-06-10
---

Orval `client: 'solid-query'` generates broken code for TanStack Query v5:
- Imports `SolidMutationOptions` — removed in v5, renamed to `CreateMutationOptions`
- Imports `MutationFunction`, `QueryFunction`, `QueryKey` from `@tanstack/solid-query` — moved to `@tanstack/query-core` in v5

No post-processing fix needed. Clean approach: use `client: 'fetch'` in Orval config, then write thin manual wrappers in `api.ts` using `@tanstack/solid-query` v5 (`createQuery`/`createMutation`). Orval handles the hard parts (types, URL building, request/response shapes). Wrapper cost is ~6 lines per endpoint.

Example wrapper:
```ts
import { createQuery } from "@tanstack/solid-query";
import { listJobs } from "~/generated/orval/jobsearch";

export function useListJobs(params: () => ListJobsParams) {
  return createQuery<JobListResponse>(() => ({
    queryKey: ["jobs", params()],
    queryFn: async () => {
      const res = await listJobs(params());
      if (res.status !== 200) throw new Error("Failed to fetch jobs");
      return res.data;
    },
  }));
}
```
