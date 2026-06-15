---
type: lesson
tags: [home-manager, pnpm, pi, global-packages, self-update]
created: 2026-06-15
updated: 2026-06-15
---

When using Home Manager together with pnpm global packages, prefer letting pnpm manage its own global layout rather than declaring globals in `home.nix`. This keeps `pi update --self` functional. Nix-owned store paths are read-only and break self-updating CLI tools. Document wanted global packages in `home.nix` comments or a separate note if reproducibility matters, but install/remove with `pnpm -g`.
