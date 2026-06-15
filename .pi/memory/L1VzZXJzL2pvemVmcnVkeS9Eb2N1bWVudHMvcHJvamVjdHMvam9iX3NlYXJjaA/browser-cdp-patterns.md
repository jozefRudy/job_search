---
type: lesson
tags: [rust, browser, cdp, brave, scraping]
created: 2026-06-15
updated: 2026-06-15
---

Connect to existing Brave/CDP session at `localhost:9222` first; on macOS, a running Brave instance launched without `--remote-debugging-port` must be quit before CDP launch. Reuse the authenticated browser context for bot-protected sites. Open inspection tabs in background (`CreateTargetParams.background(true)`). Prefer `page.wait_for_navigation()` after `goto()` and element-based waits with simple `tries`/`delay` signatures. Inspect live DOM when selectors fail; use `browser_eval` with JS extraction for verification.
