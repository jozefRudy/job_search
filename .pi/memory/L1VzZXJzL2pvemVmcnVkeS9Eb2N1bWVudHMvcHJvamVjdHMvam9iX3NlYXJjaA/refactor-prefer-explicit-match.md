---
type: preference
tags: [refactoring, rust, traits]
created: 2026-06-14
updated: 2026-06-14
---

When refactoring repeated platform-specific code, the user prefers keeping explicit match arms over boxing into `dyn Trait` mappers if the match is still needed for construction. The generic helper (`sync_apps`) is the accepted abstraction; further unification into a single `cmd.platform::new()`-style call was rejected as not cleaner.
