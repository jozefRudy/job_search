---
type: lesson
tags: [frontend, api, openapi, orval, tanstack-query]
created: 2026-06-09
updated: 2026-06-09
---

Consider experimenting with OpenAPI + Orval for auto-generating TanStack Query hooks from backend API spec. Would eliminate manual `api.ts`, `ListQuery` plumbing, and `createResource` signal wiring. Frontend would get typed `useListJobs`, `useGetJob`, `useRateJob` with caching out of the box. Most frontend-backend type synchronization pains would disappear.
