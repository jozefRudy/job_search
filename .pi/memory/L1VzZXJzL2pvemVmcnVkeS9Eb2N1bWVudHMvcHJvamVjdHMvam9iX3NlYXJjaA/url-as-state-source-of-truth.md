---
type: lesson
tags: [solidjs, frontend, url-params, state-management, architecture]
created: 2026-06-10
updated: 2026-06-10
---

URL query params are the correct source of truth for filter/list state in SPAs — not a workaround or fallback. Browser back/forward, refresh, bookmarks, and link sharing all work automatically. Any state management that duplicates URL params in signals/stores/services is redundant and creates synchronization bugs. Push filter state to URL immediately on change, read it back on render.
