---
type: lesson
tags: [nix, frontend, pnpm, flake]
created: 2026-06-10
updated: 2026-06-10
---

When frontend `pnpm` dependencies change in `frontend/package.json` or `frontend/pnpm-lock.yaml`, the `fetchPnpmDeps` hash in `flake.nix` must be updated. Workflow:

1. Set `hash = pkgs.lib.fakeHash;` temporarily in `flake.nix`
2. Run `nix build .#frontend`
3. Nix fails with hash mismatch and prints the correct hash in `got:`
4. Replace fakeHash with the correct hash from error output

The flake already has a comment documenting this next to the `fetchPnpmDeps` block.
