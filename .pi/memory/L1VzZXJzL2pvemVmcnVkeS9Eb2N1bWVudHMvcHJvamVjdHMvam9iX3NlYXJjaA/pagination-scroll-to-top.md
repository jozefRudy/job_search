---
type: lesson
tags: [frontend, solidjs, pagination, ux]
created: 2026-06-15
updated: 2026-06-15
---

For paginated lists, don't rely on router auto-scroll. Browser/router scroll restoration can skip auto-scroll when returning to a previously visited page. Explicitly `window.scrollTo({ top: 0 })` in the page-change handler to keep behavior consistent.
