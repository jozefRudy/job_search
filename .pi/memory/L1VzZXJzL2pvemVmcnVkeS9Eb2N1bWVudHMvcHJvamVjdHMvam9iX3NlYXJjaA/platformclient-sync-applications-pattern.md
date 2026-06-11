---
type: lesson
tags: [rust, traits, platforms, browser]
created: 2026-06-11
updated: 2026-06-11
---

Platform-specific sync methods (like `sync_applications`) belong in `PlatformClient` trait with default unimplemented error, not as inherent methods. Upwork implementation moved into `impl PlatformClient for UpworkScraper`. Browser should remain a method parameter, not a scraper field — lifecycle is owned by `BrowserManager`.
