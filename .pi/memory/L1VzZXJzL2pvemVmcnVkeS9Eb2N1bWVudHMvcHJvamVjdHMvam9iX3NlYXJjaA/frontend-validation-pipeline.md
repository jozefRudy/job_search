---
type: context
tags: [frontend, validation, biome]
created: 2026-06-09
updated: 2026-06-09
---

Frontend validation pipeline lives in `frontend/` dir: `pnpm typecheck && pnpm check && pnpm fix && pnpm test run && pnpm build`. Biome config copied from reddit project, adapted for this repo. No `passWithNoTests: true` — empty test suite should fail honestly until tests exist.
