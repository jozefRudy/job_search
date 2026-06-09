---
type: lesson
created: 2026-06-09
updated: 2026-06-09
---

macOS `open -a "Brave Browser" --args --remote-debugging-port=9222` silently drops `--args` when Brave already running. Cannot mix CDP and non-CDP instances. Use `pgrep -x "Brave Browser"` to detect running instance and bail fast with clear message before attempting launch. User must quit Brave, then retry so code can launch fresh with CDP.
