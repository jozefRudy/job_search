---
type: lesson
tags: [devenv, frontend, api-generation]
created: 2026-06-10
updated: 2026-06-10
---

`devenv.nix` `regen-api` script uses `pnpm -C frontend orval`, not `pnpm --dir frontend orval`. The `--dir` flag is not supported by all pnpm versions; `-C` is the correct short flag for changing directory.
