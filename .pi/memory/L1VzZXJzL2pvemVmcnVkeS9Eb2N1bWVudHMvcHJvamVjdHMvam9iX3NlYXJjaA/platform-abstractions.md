---
type: lesson
tags: [rust, platforms, api-design, traits]
created: 2026-06-15
updated: 2026-06-15
---

Platform-specific sync methods (e.g. `sync_applications`) belong in `PlatformClient` trait with default unimplemented errors. For shared sorting, use a single strongly-typed `Sort` enum with all known variants; backend maps each to SQL without platform validation, frontend renders only variants relevant to the selected platform and resets on platform switch.
