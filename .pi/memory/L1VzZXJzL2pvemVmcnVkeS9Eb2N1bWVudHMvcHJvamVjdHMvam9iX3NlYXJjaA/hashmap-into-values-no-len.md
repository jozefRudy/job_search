---
type: lesson
tags: [rust, collections]
created: 2026-06-16
updated: 2026-06-16
---

In Rust, `HashMap::into_values()` returns an iterator without `len()`. If length is needed before iteration, compute `map.len()` while the map is still borrowed, or collect into `Vec` first.
