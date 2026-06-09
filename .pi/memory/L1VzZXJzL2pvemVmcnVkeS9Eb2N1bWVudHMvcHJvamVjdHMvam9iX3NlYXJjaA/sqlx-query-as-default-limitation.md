---
type: lesson
tags: [rust, sqlx, database]
created: 2026-06-09
updated: 2026-06-09
---

`#[sqlx(flatten)]` does **not** work with the `query_as!` macro — it only works with runtime `query_as()` function. The macro generates a struct literal from SELECT columns directly and fails with "struct has no field named X" because it treats the flattened field as a single field. For one-off queries with extra columns (e.g., `COUNT(*) OVER() as total`), use `query!` (anonymous rows) and manually construct the domain struct. This is the best available option with sqlx compile-time checked macros.
