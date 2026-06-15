---
type: lesson
tags: [rust, sqlx, sqlite, types]
created: 2026-06-15
updated: 2026-06-15
---

SQLite `INTEGER PRIMARY KEY` and `COUNT(*)` infer as `i64`; keep row struct `id: i64`. `sqlx::query!.execute().rows_affected()` returns `u64`; return `u64` directly, avoid `as usize`.
