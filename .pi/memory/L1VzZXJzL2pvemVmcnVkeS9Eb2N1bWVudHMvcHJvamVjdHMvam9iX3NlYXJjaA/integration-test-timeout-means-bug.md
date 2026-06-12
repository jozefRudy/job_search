---
type: lesson
tags: [testing, debugging, browser, integration]
created: 2026-06-12
updated: 2026-06-12
---

When an ignored browser integration test times out or hangs, treat it as a bug, not a missing timeout. First reproduce individually, inspect raw responses, and fix deserialization/loop issues. Bulk `--include-ignored` failures may be caused by poisoned static locks from earlier panics, not just slow tests.
