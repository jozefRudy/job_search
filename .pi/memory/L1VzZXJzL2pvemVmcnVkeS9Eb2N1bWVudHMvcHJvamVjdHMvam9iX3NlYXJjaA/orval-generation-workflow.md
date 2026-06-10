---
type: lesson
tags: [api-generation, frontend, backend, orval]
created: 2026-06-10
updated: 2026-06-10
---

When adding new backend endpoints, always regenerate the Orval API client via `regen-api` devenv script instead of writing raw `fetch` wrappers. Keeps frontend API layer typed and consistent.
