---
type: lesson
tags: [rust, browser, design]
created: 2026-06-15
updated: 2026-06-15
---

Hide lazy initialization inside an accessor like `browser()` rather than exposing `ensure()`. Keeps `BrowserManager` lifecycle-only and guarantees init before use without caller burden. Example: `manager.browser().await?.new_tab(url).await`.
