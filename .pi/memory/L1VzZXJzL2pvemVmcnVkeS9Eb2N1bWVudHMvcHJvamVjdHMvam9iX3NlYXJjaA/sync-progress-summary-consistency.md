---
type: lesson
tags: [cli, efinancialcareers, sync-applications]
created: 2026-06-15
updated: 2026-06-15
---

eFinancialCareers `sync_applications` printed per-item progress but omitted final `state.summary()` line. Upwork prints only summary, NoFluffJobs prints both. When adding shared progress helpers, ensure all callers emit matching final summary for consistent CLI output.
