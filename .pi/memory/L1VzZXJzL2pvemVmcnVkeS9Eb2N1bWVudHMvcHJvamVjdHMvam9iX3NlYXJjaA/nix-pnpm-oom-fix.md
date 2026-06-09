---
type: lesson
tags: [nix, pnpm, macos]
created: 2026-06-08
updated: 2026-06-08
---

When packaging pnpm frontend in Nix flake on macOS, `pnpm_11` + `fetcherVersion = 4` causes OOM (SIGKILL, exit code 137). Fix: use `pnpm_10` + `nodejs-slim` + `fetcherVersion = 3` instead.
