---
type: context
tags: [orval, tanstack, solidjs, frontend]
created: 2026-06-10
updated: 2026-06-10
---

Orval issue [#3365](https://github.com/orval-labs/orval/issues/3365) tracks solid-query client incompatibility with TanStack Query v5. When fixed, can switch `client: 'fetch'` → `client: 'solid-query'` in `orval.config.js` and eliminate manual `createQuery`/`createMutation` wrappers in `api.ts`.
