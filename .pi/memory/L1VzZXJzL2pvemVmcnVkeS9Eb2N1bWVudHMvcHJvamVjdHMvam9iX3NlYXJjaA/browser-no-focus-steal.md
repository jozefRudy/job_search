---
type: lesson
tags: [browser, focus, debugging]
created: 2026-06-14
updated: 2026-06-14
---

When opening browser tabs for inspection during development, use background tabs via `browser_new_tab` or `CreateTargetParams.background(true)` rather than `window.open` or foreground activation, to avoid stealing user focus. I mistakenly used `window.open` and `browser_activate_tab` during live debugging.
