---
type: lesson
tags: [rust, terminal, output-formatting]
created: 2026-06-15
updated: 2026-06-15
---

CursorGuard cursor restore should use `eprint!("\x1B[?25h")` without a trailing newline; adding the newline in `show_cursor` injects unwanted blank lines after progress output. Add explicit newlines in the caller instead (e.g., before summary lines).
