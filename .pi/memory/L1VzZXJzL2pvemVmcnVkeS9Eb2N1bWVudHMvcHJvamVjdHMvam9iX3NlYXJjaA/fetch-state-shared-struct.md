---
type: context
tags: [fetch-state, platforms, progress-output]
created: 2026-06-15
updated: 2026-06-15
---

Created `FetchState` in `src/platforms/fetch_state.rs` and re-exported from `src/platforms/mod.rs`. Use it across providers (upwork, nofluffjobs, efinancialcareers) to track checked/new/existing counts and print consistent progress/summary lines. Summary format: `Total checked: N (X new, Y existing)`.
