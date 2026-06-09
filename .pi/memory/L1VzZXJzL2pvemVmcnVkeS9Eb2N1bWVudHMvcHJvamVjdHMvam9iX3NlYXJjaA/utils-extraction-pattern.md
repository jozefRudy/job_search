---
type: context
tags: [frontend, refactoring, solidjs]
created: 2026-06-09
updated: 2026-06-09
---

Pure helper functions (e.g. `fmtRelative`, `ratingEmoji`, `ratingClass`) belong in `lib/utils.ts`, not duplicated across components. When refactoring, `grep` for duplicates before moving.
