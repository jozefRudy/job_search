# Project Memory


## lesson

- [[agents-md-location]] - Project-wide agent instructions live in `.pi/APPEND_SYSTEM.md` (pi-native, auto-
- [[brave-cdp-macos-launch]] - macOS `open -a "Brave Browser" --args --remote-debugging-port=9222` silently dro
- [[browser-eval-over-screenshots]] - When model doesn't support image rendering, use `browser_eval` with JS extractio
- [[copy-ui-components-checklist]] - When copying UI components from another project, always verify: 1) missing depen
- [[dont-strip-platform-features]] - When refactoring to remove platform-specific leakage from generic types, don't a
- [[frontend-api-mismatch]] - Frontend `api.ts` must unwrap server response shape. Server `list_jobs` returns 
- [[frontend-ui-pure-copies]] - When copying UI components from `../reddit/frontend-solid/src/components/ui/`, u
- [[nix-pnpm-hash-update]] - When frontend `pnpm` dependencies change in `frontend/package.json` or `frontend
- [[nix-pnpm-oom-fix]] - When packaging pnpm frontend in Nix flake on macOS, `pnpm_11` + `fetcherVersion 
- [[openapi-orval-experiment]] - Consider experimenting with OpenAPI + Orval for auto-generating TanStack Query h
- [[orval-solid-query-v5-incompatibility]] - Orval `client: 'solid-query'` generates broken code for TanStack Query v5:
- [[pagination-single-query-pattern]] - For paginated APIs, use `COUNT(*) OVER() as total` in the same query instead of 
- [[platform-specific-sort-pattern]] - For platform-specific sorting in shared API, use a single strongly-typed `Sort` 
- [[pnpm-workspaces-single-app]] - Single-app frontend does not need `pnpm-workspace.yaml`. Remove it. `pnpm.onlyBu
- [[solidjs-nullable-filter-url-state]] - Clean pattern for filter state persisted in URL with SolidJS: use `useSearchPara
- [[solidjs-presentation-helpers-undefined]] - In SolidJS projects, presentation-layer helpers (`fmtRelative`, `ratingEmoji`, `
- [[solidjs-url-filter-state]] - For persistent filter state in SolidJS: use `@solidjs/router`'s `useSearchParams
- [[sqlx-query-as-default-limitation]] - `#[sqlx(flatten)]` does **not** work with the `query_as!` macro — it only works 
- [[tanstack-solidjs-structural-sharing]] - When using TanStack Query v5 with SolidJS, always set `structuralSharing: false`

## context

- [[devenv-e2e-process]] - Document end-to-end process in `.pi/APPEND_SYSTEM.md`: `devenv up` starts backen
- [[frontend-validation-pipeline]] - Frontend validation pipeline lives in `frontend/` dir: `pnpm typecheck && pnpm c
- [[tanstack-solidjs-pattern]] - Idiomatic SolidJS + TanStack Query v5 pattern for reactive data fetching:
- [[ts-rs-generated-types]] - Frontend TypeScript API types are auto-generated from Rust via `ts-rs`. `#[ts(ex
- [[utils-extraction-pattern]] - Pure helper functions (e.g. `fmtRelative`, `ratingEmoji`, `ratingClass`) belong 
- [[web-server-design]] - Frontend stack: SolidJS + Tailwind + DaisyUI, built locally in `frontend/` dir a
