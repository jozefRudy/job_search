---
type: context
tags: [devenv, e2e, frontend, backend]
created: 2026-06-09
updated: 2026-06-09
---

Document end-to-end process in `.pi/APPEND_SYSTEM.md`: `devenv up` starts backend (`cargo run -- serve`) + frontend (`pnpm start`) together. Frontend proxies `/api` to `localhost:8080`. Verify key flows in browser after UI changes.
