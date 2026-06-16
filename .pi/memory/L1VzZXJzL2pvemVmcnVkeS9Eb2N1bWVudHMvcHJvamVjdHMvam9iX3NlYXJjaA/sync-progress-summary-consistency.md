---
type: lesson
tags: [cli, efinancialcareers, sync-applications]
created: 2026-06-15
updated: 2026-06-16
---

Current code centralizes final summary in `src/main.rs` (`sync_apps`/`fetch_and_store` print `state.summary()` on success). Provider implementations only return `FetchState`; do not add per-provider `println!(summary)`. This keeps output consistent across all three providers.
