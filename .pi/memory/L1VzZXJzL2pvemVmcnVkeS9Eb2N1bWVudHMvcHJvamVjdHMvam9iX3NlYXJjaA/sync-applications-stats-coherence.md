---
type: lesson
tags: [stats, sync, bugfix]
created: 2026-06-16
updated: 2026-06-16
---

When syncing applications, increment `new`/`existing` stats exactly once per proposal and avoid double-counting already-applied rows. For Upwork that meant removing a second `inc_existing()` inside the already-applied branch and incrementing `new` immediately on job insert rather than after the applied check.
