# Project Memory


## lesson

- [[agents-md-location]] - Project-wide agent instructions live in `.pi/APPEND_SYSTEM.md` (pi-native, auto-
- [[api-test-catches-reactivity-regressions]] - API-level tests with mocked fetch are essential for catching silent reactivity r
- [[browser-cdp-patterns]] - Connect to existing Brave/CDP session at `localhost:9222` first; on macOS, a run
- [[browser-spa-wait-strategy]] - For SPA-rendered job boards (Upwork, NoFluffJobs, eFinancialCareers), prefer ele
- [[budget-model]] - Centralized `Budget` enum in `src/models.rs` has `Range`/`Single` variants. `Bud
- [[clippy-all-targets-validation]] - Validation command should include `--all-targets` for clippy so tests and integr
- [[cursor-guard-no-newline]] - CursorGuard cursor restore should use `eprint!("\x1B[?25h")` without a trailing 
- [[deserializer-shape-normalization]] - When scraping external APIs, do shape-normalization in explicit `Raw*` structs p
- [[detail-list-shared-grid]] - For label/value rows in detail views, prefer a shared wrapper grid (`DetailList`
- [[detail-posted-at-non-optional]] - When a scraped field like `posted_at` is the source of truth for a non-optional 
- [[dont-strip-platform-features]] - When refactoring to remove platform-specific leakage from generic types, don't a
- [[efinancialcareers-detail-selectors]] - eFinancialCareers detail page exposes company and location via header selectors 
- [[efinancialcareers-scraping]] - Use Brave/CDP for search (bot protection blocks curl). Matched results live in `
- [[file-move-pattern]] - When moving files, prefer `mv` (bash) + `edit` for import path updates over `wri
- [[frontend-presentation-helpers]] - Pure presentation helpers (`fmtRelative`, `ratingEmoji`, `ratingClass`) live in 
- [[frontend-ui-copying]] - When copying UI components from `../reddit/frontend-solid/src/components/ui/`, u
- [[hashmap-into-values-no-len]] - In Rust, `HashMap::into_values()` returns an iterator without `len()`. If length
- [[integration-test-timeout-means-bug]] - When an ignored browser integration test times out or hangs, treat it as a bug, 
- [[job-created-at-semantics]] - In this project, `Job::created_at` and the DB `jobs.created_at` column store the
- [[job-id-non-optional]] - DB-assigned IDs should be non-optional in API models (`id: i64`, not `Option<i64
- [[new-platform-checklist]] - For a new job board, start with a CLI-only spike: confirm search cards, paginati
- [[nix-pnpm-hash-update]] - When frontend `pnpm` dependencies change in `frontend/package.json` or `frontend
- [[nix-pnpm-oom-fix]] - When packaging pnpm frontend in Nix flake on macOS, `pnpm_11` + `fetcherVersion 
- [[nix-vs-pnpm-global-packages]] - When using Home Manager together with pnpm global packages, prefer letting pnpm 
- [[nofluffjobs-sync]] - NoFluffJobs applications sync uses `/api/candidates/my-applications`, paginated,
- [[orval-api-generation]] - API types and fetch clients are generated from backend OpenAPI via Orval (`clien
- [[orval-optional-properties]] - Orval honors the OpenAPI `required` array. Non-`Option<T>` Rust fields with no `
- [[output-label-match-actual-work]] - When output shows counts, prefer language that matches work done. For fetch/upda
- [[pagination-patterns]] - Backend: use `COUNT(*) OVER() as total` in the same query to avoid duplicated WH
- [[plan-before-implement]] - When the user says 'first verify/research before starting', provide a concise pl
- [[platform-abstractions]] - Platform-specific sync methods (e.g. `sync_applications`) belong in `PlatformCli
- [[pnpm-workspaces-single-app]] - Single-app frontend does not need `pnpm-workspace.yaml`. Remove it. `pnpm.onlyBu
- [[regen-api-fallback]] - The `devenv.nix` `regen-api` script is the preferred way to regenerate Orval cli
- [[solidjs-filter-url-state]] - URL query params are source of truth for list/filter state in SolidJS. Use `useS
- [[sqlite-foreign-keys-default-off]] - SQLite foreign keys are disabled by default and must be enabled per connection v
- [[sqlx-numeric-types]] - SQLite `INTEGER PRIMARY KEY` and `COUNT(*)` infer as `i64`; keep row struct `id:
- [[sqlx-query-as-default-limitation]] - `#[sqlx(flatten)]` does **not** work with the `query_as!` macro — it only works 
- [[sync-applications-pipeline-pattern]] - All three providers share the same `sync_applications` pipeline: collect platfor
- [[sync-applications-stats-coherence]] - When syncing applications, increment `new`/`existing` stats exactly once per pro
- [[sync-progress-summary-consistency]] - Current code centralizes final summary in `src/main.rs` (`sync_apps`/`fetch_and_
- [[tanstack-query-over-engineering]] - Don't over-engineer TanStack Query cache invalidation. User prefers short, maint
- [[tanstack-solidjs-reactivity]] - Use TanStack Query v5 with SolidJS: set `structuralSharing: false` on every `cre
- [[upwork-hidden-budget-marker]] - Upwork detail pages may hide hourly budgets client-side. The server still emits 

## preference

- [[refactor-prefer-explicit-match]] - When refactoring repeated platform-specific code, the user prefers keeping expli
- [[update-command-output-spacing]] - When chaining multiple `jobsearch update` commands, print a blank line before ea

## context

- [[db-set-applied-signature]] - `Db::set_applied` should take explicit non-optional `applied_at: NaiveDateTime` 
- [[devenv-scripts]] - `devenv up` starts backend (`cargo run -- serve`) + frontend (`pnpm start`) toge
- [[fetch-state-shared-struct]] - `FetchState` in `src/platforms/fetch_state.rs` tracks `new`/`existing` counts an
- [[frontend-validation-pipeline]] - Frontend validation pipeline lives in `frontend/` dir: `pnpm typecheck && pnpm c
- [[provider-host-check-guard]] - Each `fetch_with_browser`/`sync_applications` implementation starts by checking 
- [[skip-detail-fetch-existing-jobs]] - In fetch loops, skip detail fetch for existing jobs via `db.find_job_id()` befor
- [[web-server-design]] - Frontend stack: SolidJS + Tailwind + DaisyUI, built locally in `frontend/` dir a
