---
type: lesson
tags: [rust, sqlx, sqlite]
created: 2026-06-15
updated: 2026-06-15
---

SQLite `INTEGER PRIMARY KEY` and `COUNT(*)` are inferred as `i64` by sqlx. Keep row struct `id: i64` unless schema enforces unsigned and downstream needs `u64`.
