# Project Memory


## lesson

- [[agents-md-location]] - Project-wide agent instructions live in `.pi/APPEND_SYSTEM.md` (pi-native, auto-
- [[angular-url-state-lesson]] - Angular projects often over-engineer list/filter state with services, resolvers,
- [[api-test-catches-reactivity-regressions]] - API-level tests with mocked fetch are essential for catching silent reactivity r
- [[brave-cdp-macos-launch]] - macOS `open -a "Brave Browser" --args --remote-debugging-port=9222` silently dro
- [[browser-eval-over-screenshots]] - When model doesn't support image rendering, use `browser_eval` with JS extractio
- [[browser-no-focus-steal]] - When opening browser tabs for inspection during development, use background tabs
- [[browser-scraping-bot-protection]] - For browser-driven scrapers behind bot protection (eFinancialCareers), reuse the
- [[browser-test-debug-inspect-live]] - When debugging browser integration test failures, prefer live browser inspection
- [[browser-wait-helpers-design]] - When extracting browser wait helpers, keep signatures simple: `tries: Option<u32
- [[budget-model-centralized]] - Budget model is a central enum (`Range`/`Single`) in `src/models.rs` with `Displ
- [[budget-parse-regex-pattern]] - Budget parsing prefers a single regex that handles separators with optional whit
- [[copy-ui-components-checklist]] - When copying UI components from another project, always verify: 1) missing depen
- [[deserializer-shape-normalization]] - When scraping external APIs, do shape-normalization in explicit `Raw*` structs p
- [[detail-list-shared-grid]] - For label/value rows in detail views, prefer a shared wrapper grid (`DetailList`
- [[devenv-regen-api-script]] - `devenv.nix` `regen-api` script uses `pnpm -C frontend orval`, not `pnpm --dir f
- [[dont-strip-platform-features]] - When refactoring to remove platform-specific leakage from generic types, don't a
- [[efinancialcareers-applications-sync]] - eFinancialCareers My Jobs popup descriptions come from `https://job.efinancialca
- [[efinancialcareers-empty-results-selector]] - eFinancialCareers zero-results page renders `efc-empty-job-search-results-wrappe
- [[efinancialcareers-filter-url-pattern]] - eFinancialCareers filter URL pattern: multi-value filters use `|` separator (URL
- [[efinancialcareers-search-results-selector]] - On eFinancialCareers search pages, matched results are rendered inside `<efc-job
- [[efinancialcareers-sync-use-batch-api]] - eFinancialCareers application sync should use the batch API (`job.efinancialcare
- [[efinancialcareers-total-unreliable]] - eFinancialCareers exposes a `transferredData()` script and a visible heading cou
- [[file-move-pattern]] - When moving files, prefer `mv` (bash) + `edit` for import path updates over `wri
- [[frontend-api-mismatch]] - Frontend `api.ts` must unwrap server response shape. Server `list_jobs` returns 
- [[frontend-ui-pure-copies]] - When copying UI components from `../reddit/frontend-solid/src/components/ui/`, u
- [[integration-test-timeout-means-bug]] - When an ignored browser integration test times out or hangs, treat it as a bug, 
- [[new-platform-integration-spike]] - When integrating a new job board, scrape enough detail to make the listing usefu
- [[new-platform-variant-update-checklist]] - When a new `Platform` variant is added, also update `db.rs` test helper `test_jo
- [[nix-pnpm-hash-update]] - When frontend `pnpm` dependencies change in `frontend/package.json` or `frontend
- [[nix-pnpm-oom-fix]] - When packaging pnpm frontend in Nix flake on macOS, `pnpm_11` + `fetcherVersion 
- [[nofluffjobs-auth-token-change]] - NoFluffJobs sync auth changed: `nfj_salt` cookie replaced by `nfj_token=<session
- [[openapi-orval-experiment]] - Consider experimenting with OpenAPI + Orval for auto-generating TanStack Query h
- [[orval-generation-workflow]] - When adding new backend endpoints, always regenerate the Orval API client via `r
- [[orval-no-manual-edits]] - Orval-generated schemas should not be edited by hand. When backend schema change
- [[orval-solid-query-v5-incompatibility]] - Orval `client: 'solid-query'` was broken for TanStack Query v5 but is now FIXED 
- [[pagination-single-query-pattern]] - For paginated APIs, use `COUNT(*) OVER() as total` in the same query instead of 
- [[plan-before-implement]] - When the user says 'first verify/research before starting', provide a concise pl
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
- [[efinancialcareers-applications-sync-todo]] - eFinancialCareers applications sync TODO: scrape `https://www.efinancialcareers.
- [[frontend-validation-pipeline]] - Frontend validation pipeline lives in `frontend/` dir: `pnpm typecheck && pnpm c
- [[nofluffjobs-sync-plan]] - NoFluffJobs applications sync uses `/api/candidates/my-applications` with HMAC a
- [[orval-solid-query-pattern]] - With orval `client: 'solid-query'`, use generated hooks for mutations but custom
- [[orval-solid-query-v5-watch]] - Orval issue [#3365](https://github.com/orval-labs/orval/issues/3365) tracks soli
- [[tanstack-solidjs-pattern]] - Idiomatic SolidJS + TanStack Query v5 pattern for reactive data fetching:
- [[ts-rs-generated-types]] - Frontend TypeScript API types are auto-generated from Rust via `ts-rs`. `#[ts(ex
- [[utils-extraction-pattern]] - Pure helper functions (e.g. `fmtRelative`, `ratingEmoji`, `ratingClass`) belong 
- [[web-server-design]] - Frontend stack: SolidJS + Tailwind + DaisyUI, built locally in `frontend/` dir a
