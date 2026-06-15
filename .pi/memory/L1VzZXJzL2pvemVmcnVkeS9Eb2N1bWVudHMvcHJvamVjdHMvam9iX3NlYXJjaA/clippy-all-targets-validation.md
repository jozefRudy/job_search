---
type: lesson
tags: [rust, clippy, validation]
created: 2026-06-15
updated: 2026-06-15
---

Validation command should include `--all-targets` for clippy so tests and integration tests are linted too. Updated `.pi/APPEND_SYSTEM.md` command: `cargo build && cargo clippy --all-targets -- -D warnings && cargo test && cargo fmt`.
