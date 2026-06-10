---
type: lesson
tags: [solidjs, angular, url-params, state-management]
created: 2026-06-10
updated: 2026-06-10
---

In SolidJS (and modern SPAs), persist list/filter state in URL query params via `useSearchParams` — not in cached service/store like Angular patterns. URL params survive back/forward, refresh, and are shareable. `navigate(-1)` for back buttons preserves state automatically. No need for screen-level state caching, resolver guards, or shared services.
