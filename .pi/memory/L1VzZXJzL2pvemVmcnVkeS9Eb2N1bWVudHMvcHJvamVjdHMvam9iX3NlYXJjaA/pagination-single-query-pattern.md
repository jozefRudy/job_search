---
type: lesson
tags: [rust, sqlx, pagination, api-design]
created: 2026-06-09
updated: 2026-06-09
---

For paginated APIs, use `COUNT(*) OVER() as total` in the same query instead of separate `SELECT COUNT(*)` query. Eliminates duplicated WHERE logic and extra DB round-trip. Returns `(Vec<Job>, i64)` from one method.
