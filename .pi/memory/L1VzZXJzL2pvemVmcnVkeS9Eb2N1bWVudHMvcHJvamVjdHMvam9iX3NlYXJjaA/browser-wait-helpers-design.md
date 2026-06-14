---
type: lesson
tags: [rust, browser, refactoring]
created: 2026-06-14
updated: 2026-06-14
---

When extracting browser wait helpers, keep signatures simple: `tries: Option<u32>` and `delay: Option<Duration>` with sensible defaults (30 tries, 500ms). Use `&[&str]` for selectors so one `wait_for_element` handles single and multiple selectors; drop `wait_for_element_any` and `wait_for_js` if they have no callers. Prefer element-based waits over JS predicate waits when selectors are available.
