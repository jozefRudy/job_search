# Project Memory


## lesson

- [[brave-cdp-macos-launch]] - macOS `open -a "Brave Browser" --args --remote-debugging-port=9222` silently dro
- [[browser-eval-over-screenshots]] - When model doesn't support image rendering, use `browser_eval` with JS extractio
- [[copy-ui-components-checklist]] - When copying UI components from another project, always verify: 1) missing depen
- [[frontend-api-mismatch]] - Frontend `api.ts` must unwrap server response shape. Server `list_jobs` returns 
- [[frontend-ui-pure-copies]] - When copying UI components from `../reddit/frontend-solid/src/components/ui/`, u
- [[nix-pnpm-oom-fix]] - When packaging pnpm frontend in Nix flake on macOS, `pnpm_11` + `fetcherVersion 
- [[pagination-single-query-pattern]] - For paginated APIs, use `COUNT(*) OVER() as total` in the same query instead of 
- [[pnpm-workspaces-single-app]] - Single-app frontend does not need `pnpm-workspace.yaml`. Remove it. `pnpm.onlyBu
- [[sqlx-query-as-default-limitation]] - `#[sqlx(flatten)]` does **not** work with the `query_as!` macro — it only works 

## context

- [[web-server-design]] - Frontend stack: SolidJS + Tailwind + DaisyUI, built locally in `frontend/` dir a
