---
type: context
tags: [frontend, rust, ts-rs, types]
created: 2026-06-09
updated: 2026-06-09
---

Frontend TypeScript API types are auto-generated from Rust via `ts-rs`. `#[ts(export)]` types in `src/models.rs` write to `frontend/src/generated/` during `cargo test`. Never edit generated files manually. Biome ignores `src/generated`.

**Rule:** Export query structs from Rust (e.g. `ListQuery` with `Option<Platform>` / `Option<Rating>`) — they map to `Platform | null` / `Rating | null` in TS. Never create frontend-only wrapper types like `PlatformFilter` / `RatingFilter`. Single source of truth, no string parsing in handlers.
