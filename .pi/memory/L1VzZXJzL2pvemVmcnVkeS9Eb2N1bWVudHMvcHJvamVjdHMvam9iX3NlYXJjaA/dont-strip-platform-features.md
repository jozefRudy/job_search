---
type: lesson
tags: [refactoring, frontend, platform-specific, design]
created: 2026-06-09
updated: 2026-06-09
---

When refactoring to remove platform-specific leakage from generic types, don't also remove the corresponding UI feature. Prefer keeping the capability and making the boundary clean: separate platform-specific types, validate at runtime, or scope options per platform in the frontend.
