---
type: lesson
tags: [api-design, sorting, platform-specific, frontend, rust]
created: 2026-06-09
updated: 2026-06-09
---

For platform-specific sorting in shared API, use a single strongly-typed `Sort` enum with all known variants (e.g. `Created`, `UpworkViewed`). Backend maps each variant to SQL via `order_by_sql()` and does NOT validate platform compatibility. Frontend uses the generated `ListQuery` type directly in `createResource`: signal function returns `ListQuery`, fetcher passes it to `listJobs(query: ListQuery)`. Frontend builds a `PLATFORM_SORTS` dictionary per platform from the exported type and renders only the sorts relevant to the selected platform. Reset sort to default when switching away from a platform that supports the current sort. Hand-crafted API requests with mismatched platform/sort are harmless (sorts by null column).
