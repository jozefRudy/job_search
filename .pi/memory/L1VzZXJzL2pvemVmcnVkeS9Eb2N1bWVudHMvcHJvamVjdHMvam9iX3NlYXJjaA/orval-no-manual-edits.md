---
type: lesson
tags: [frontend, orval, openapi]
created: 2026-06-14
updated: 2026-06-14
---

Orval-generated schemas should not be edited by hand. When backend schema changes (e.g. adding fields to `UpworkJobDetail`/`NoFluffJobDetail`), running `devenv shell regen-api` updates `frontend/src/generated/orval/jobsearch.schemas.ts` automatically. Frontend changes should rely on regenerated types rather than manual edits.
