---
type: context
tags: [browser, scraping, platforms]
created: 2026-06-16
updated: 2026-06-16
---

Each `fetch_with_browser`/`sync_applications` implementation starts by checking `browser.get_page_urls()` for a tab whose host contains the provider domain. Fast-fail with a clear login message if absent. This reuses the user's existing authenticated session instead of automating login.
