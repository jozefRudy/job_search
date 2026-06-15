---
type: lesson
tags: [frontend, backend, api, openapi, orval, tanstack-query]
created: 2026-06-15
updated: 2026-06-15
---

API types and fetch clients are generated from backend OpenAPI via Orval (`client: 'solid-query'`). Run `devenv shell regen-api` after any backend endpoint/schema change. Never edit `frontend/src/generated/orval/*.ts` manually.

Orval `client: 'solid-query'` works with TanStack Query v5 as of v8.16.0. Generated hooks are used directly for mutations; queries use generated `get*QueryKey` + fetch functions wrapped in manual `createQuery` because the raw response needs status unwrapping. Always set `structuralSharing: false` and render data with `<Show keyed when={data}>`.
