---
type: lesson
tags: [frontend, orval, devenv, workflow]
created: 2026-06-16
updated: 2026-06-16
---

The `devenv.nix` `regen-api` script is the preferred way to regenerate Orval clients, but it can abort. Fallback: start `cargo run -- serve`, wait for `/api/openapi.json`, then run `pnpm orval` in `frontend/`.
