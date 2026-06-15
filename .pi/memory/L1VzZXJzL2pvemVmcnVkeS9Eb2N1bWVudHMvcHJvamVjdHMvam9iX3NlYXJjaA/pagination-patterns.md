---
type: lesson
tags: [rust, sqlx, frontend, pagination]
created: 2026-06-15
updated: 2026-06-15
---

Backend: use `COUNT(*) OVER() as total` in the same query to avoid duplicated WHERE logic and an extra round-trip. Frontend: do not rely on router auto-scroll; call `window.scrollTo({ top: 0 })` explicitly in the page-change handler.
