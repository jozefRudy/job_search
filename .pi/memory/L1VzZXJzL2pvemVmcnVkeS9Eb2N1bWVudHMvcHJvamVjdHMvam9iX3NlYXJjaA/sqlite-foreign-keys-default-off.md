---
type: lesson
tags: [sqlite, sqlx, foreign-keys, migrations]
created: 2026-06-12
updated: 2026-06-12
---

SQLite foreign keys are disabled by default and must be enabled per connection via `PRAGMA foreign_keys = ON`. sqlx `SqliteConnectOptions` has `.foreign_keys(true)` to enforce. `ON DELETE CASCADE` in schema alone is not enough if FK enforcement is off.
