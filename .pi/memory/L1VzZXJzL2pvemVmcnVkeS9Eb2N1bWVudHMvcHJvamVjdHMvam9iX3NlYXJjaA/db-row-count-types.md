---
type: lesson
tags: [rust, sqlx, db]
created: 2026-06-15
updated: 2026-06-15
---

Row counts from `sqlx::query!.execute().rows_affected()` are `u64`. Return `u64` directly instead of `as usize` to avoid casts and truncation on 32-bit targets. Example: `Ok(rows.rows_affected())` with `Result<u64>`.
