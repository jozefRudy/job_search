---
type: context
tags: [fetch-state, platforms, progress-output]
created: 2026-06-15
updated: 2026-06-16
---

`FetchState` in `src/platforms/fetch_state.rs` tracks `new`/`existing` counts and renders `\r    Progress: ...` lines via `progress_line()`. Caller in `main.rs` prints final `state.summary()`. Use inside a `CursorGuard` block so carriage-return progress lines restore cursor visibility on panic/drop.
