# Project Memory


## lesson

- [[agents-md-location]] - Project-wide agent instructions live in `.pi/APPEND_SYSTEM.md` (pi-native, auto-
- [[angular-url-state-lesson]] - Angular projects often over-engineer list/filter state with services, resolvers,
- [[api-test-catches-reactivity-regressions]] - API-level tests with mocked fetch are essential for catching silent reactivity r
- [[brave-cdp-macos-launch]] - macOS `open -a "Brave Browser" --args --remote-debugging-port=9222` silently dro
- [[browser-eval-over-screenshots]] - When model doesn't support image rendering, use `browser_eval` with JS extractio
- [[copy-ui-components-checklist]] - When copying UI components from another project, always verify: 1) missing depen
- [[devenv-regen-api-script]] - `devenv.nix` `regen-api` script uses `pnpm -C frontend orval`, not `pnpm --dir f
- [[dont-strip-platform-features]] - When refactoring to remove platform-specific leakage from generic types, don't a
- [[efinancialcareers-filter-url-pattern]] - eFinancialCareers filter URL pattern: multi-value filters use `|` separator (URL
- [[file-move-pattern]] - When moving files, prefer `mv` (bash) + `edit` for import path updates over `wri
- [[frontend-api-mismatch]] - Frontend `api.ts` must unwrap server response shape. Server `list_jobs` returns 
- [[frontend-ui-pure-copies]] - When copying UI components from `../reddit/frontend-solid/src/components/ui/`, u
- [[nix-pnpm-hash-update]] - When frontend `pnpm` dependencies change in `frontend/package.json` or `frontend
- [[nix-pnpm-oom-fix]] - When packaging pnpm frontend in Nix flake on macOS, `pnpm_11` + `fetcherVersion 
- [[openapi-orval-experiment]] - Consider experimenting with OpenAPI + Orval for auto-generating TanStack Query h
- [[orval-generation-workflow]] - When adding new backend endpoints, always regenerate the Orval API client via `r
- [[orval-solid-query-v5-incompatibility]] - Orval `client: 'solid-query'` was broken for TanStack Query v5 but is now FIXED 
- [[pagination-single-query-pattern]] - For paginated APIs, use `COUNT(*) OVER() as total` in the same query instead of 
- [[platform-specific-sort-pattern]] - For platform-specific sorting in shared API, use a single strongly-typed `Sort` 
- [[platformclient-sync-applications-pattern]] - Platform-specific sync methods (like `sync_applications`) belong in `PlatformCli
- [[pnpm-workspaces-single-app]] - Single-app frontend does not need `pnpm-workspace.yaml`. Remove it. `pnpm.onlyBu
- [[solidjs-nullable-filter-url-state]] - For persistent filter state in SolidJS: use `useSearchParams` + Zod `.nullable()
- [[solidjs-presentation-helpers-undefined]] - In SolidJS projects, presentation-layer helpers (`fmtRelative`, `ratingEmoji`, `
- [[solidjs-url-filter-state]] - For persistent filter state in SolidJS: use `@solidjs/router`'s `useSearchParams
- [[solidjs-vs-angular-state-persistence]] - In SolidJS (and modern SPAs), persist list/filter state in URL query params via 
- [[sqlite-foreign-keys-default-off]] - SQLite foreign keys are disabled by default and must be enabled per connection v
- [[sqlx-query-as-default-limitation]] - `#[sqlx(flatten)]` does **not** work with the `query_as!` macro — it only works 
- [[tanstack-query-over-engineering]] - Don't over-engineer TanStack Query cache invalidation. User prefers short, maint
- [[tanstack-solidjs-structural-sharing]] - When using TanStack Query v5 with SolidJS, always set `structuralSharing: false`
- [[url-as-state-source-of-truth]] - URL query params are the correct source of truth for filter/list state in SPAs —

## context

- [[db-set-applied-signature]] - `Db::set_applied` should take explicit non-optional `applied_at: NaiveDateTime` 
- [[devenv-e2e-process]] - Document end-to-end process in `.pi/APPEND_SYSTEM.md`: `devenv up` starts backen
- [[frontend-validation-pipeline]] - Frontend validation pipeline lives in `frontend/` dir: `pnpm typecheck && pnpm c
- [[orval-solid-query-pattern]] - With orval `client: 'solid-query'`, use generated hooks for mutations but custom
- [[orval-solid-query-v5-watch]] - Orval issue [#3365](https://github.com/orval-labs/orval/issues/3365) tracks soli
- [[tanstack-solidjs-pattern]] - Idiomatic SolidJS + TanStack Query v5 pattern for reactive data fetching:
- [[ts-rs-generated-types]] - Frontend TypeScript API types are auto-generated from Rust via `ts-rs`. `#[ts(ex
- [[utils-extraction-pattern]] - Pure helper functions (e.g. `fmtRelative`, `ratingEmoji`, `ratingClass`) belong 
- [[web-server-design]] - Frontend stack: SolidJS + Tailwind + DaisyUI, built locally in `frontend/` dir a
