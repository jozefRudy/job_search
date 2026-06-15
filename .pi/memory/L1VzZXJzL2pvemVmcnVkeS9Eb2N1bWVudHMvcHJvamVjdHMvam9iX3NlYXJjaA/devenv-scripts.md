---
type: context
tags: [devenv, frontend, backend, api-generation, e2e]
created: 2026-06-15
updated: 2026-06-15
---

`devenv up` starts backend (`cargo run -- serve`) + frontend (`pnpm start`) together; frontend proxies `/api` to `localhost:8080`. Verify key flows in browser after UI changes. Regenerate API client with `devenv shell regen-api`, which uses `pnpm -C frontend orval` (not `--dir frontend`).
