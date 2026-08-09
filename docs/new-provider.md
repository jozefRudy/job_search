# Adding a New Job Provider

Generic checklist + caveats learned from existing providers (Upwork, NoFluffJobs,
eFinancialCareers, LinkedIn, WorkAtAStartup, Wellfound, Reddit, Hacker News).

## Implementation workflow

Use the agent's 3-phase approach (TODOs → stubs → implementation, approval
per phase). Keeps URL-semantics and type-shape decisions (caveats
below) visible before any scraping code exists.

## Checklist

1. **Platform enum** — add variant in `src/models.rs` `Platform`, plus its
   `Display` / `FromStr` arms (kebab-case name, e.g. `"wellfound"`).
2. **Module** — `src/platforms/<name>.rs` implementing `PlatformClient`:
   - `name()` — same kebab-case string.
   - `fetch_with_browser(&Browser, &Db, url, pause_ms) -> Result<FetchState>`.
   - Pure mapping fn `hit_to_job` (or equivalent) — unit-testable without browser.
3. **Raw + detail types** — `Raw<Name>Job` (deserialized from page/JSON) and
   `<Name>JobDetail` stored in `Data::<Name> { detail }` (models.rs). New
   `Data::` variant = exhaustive match arms in several places (`company()`,
   searchable-text, `db.rs`, `embeddings_store.rs`). Compiler finds them,
   but `grep "Data::Reddit"`-style sites proactively to be sure.
4. **Wiring** — `src/main.rs`: provider subcommand match arm that builds the
   client from `Settings` and bails with `no URLs configured for <name> in
   jobsearch.toml` when empty.
5. **Config** — default URLs in `Settings::sample()` (`src/config.rs`) and
   `jobsearch.toml`.
6. **Tests** — unit tests on recorded fixtures (no live network); browser
   integration tests as `#[ignore]`, run via `cargo test -- --include-ignored`.
   Write the integration test early and run it against a live session
   *before* finalizing field types — live data surfaces things fixtures
   can't (e.g. Algolia hits with explicit `min_experience: null`).
7. **Frontend** — run `regen-api` after the `Platform` enum change, then add
   the label + sort map entry in `JobList.tsx` and a detail card in
   `JobDetail.tsx`.
8. **README** — add provider link entry.
9. **Validation** — `cargo build && cargo clippy --all-targets && cargo test
   && cargo fmt`; frontend `pnpm typecheck && pnpm check && pnpm test run &&
   pnpm build`.

## Caveats learned the hard way

### Data acquisition — prefer structured data, DOM last

- **JSON API / GraphQL first** — wellfound reads Apollo GraphQL results
  (`JobListingSearchResult`, companies via `StartupResult.highlightedJobListings`
  reverse map); hackernews and workatastartup hit Algolia JSON APIs; linkedin
  uses the Voyager API; efinancialcareers reads facade JSON.
- **Session-bound APIs** — WaaS Algolia key is per-session (issued only to
  logged-in sessions): extract it in-tab via
  `performance.getEntriesByType('resource')`. Reddit's anonymous `.json`
  returns 403 — fetch inside the user's browser session.
- **DOM scraping last resort** — nofluffjobs cards.
- **Unstructured text → LLM extraction** — HN/reddit "Who's Hiring"
  megathreads: top-level comments are jobs, fields extracted via LLM.

### Anti-bot challenges

Use `wait_for_with_challenge_recovery` (Cloudflare etc.) instead of plain
waits on protected sites (linkedin, upwork) — challenges appear
unpredictably and must be waited out, not treated as failure.

### Nullable JSON fields

`#[serde(default)]` does NOT cover explicit JSON `null` — use `Option<T>`
for any field that can be null, and keep `Option` end-to-end. Don't "fix at
boundary" with `unwrap_or(0)`: that conflates "new grads ok" with "unknown".

### Pagination

- Verify the pagination mechanism from the captured response, not from URL
  params guess (cursor-based vs page param — don't assume).
- No early-stop heuristics when full scan is cheap (hits complete, upsert
  idempotent). Simpler = more robust.

### Identity for dedup

Use the platform's own stable ID as `external_id` when available (hackernews
`objectID`, linkedin job id, reddit comment `name`, wellfound hit id) — no
transformation. Normalize only when the platform emits the same job under
varying URL/ID forms, or duplicates get stored. Upwork example
(`normalize_upwork_external_id` / `normalize_upwork_url`): live IDs appear
both as bare digits (`data-ev-job-uid` attribute) and canonical
`~02{digits}` in URLs — normalize IDs to `~02{digits}` (presence of `~`
decides the form) and rebuild canonical URLs
`https://www.upwork.com/jobs/~02{digits}` stripped of slug, query params
and referrer.

### Model region as a type, not a string

Wellfound maps a `Region` enum to URL slugs (`wf_location_slug`) and
validates the configured URL's slug against settings. Strings invite
mismatched or unsupported values; enums make invalid regions
unrepresentable.

### URL semantics differ subtly per platform — validate, don't trust config

Wellfound example: `/role/r/<role>` is the *global remote* listing with **no
region filter**; `/role/l/<role>/<region>` is the *location* listing that
includes remote jobs tied to that region. They look interchangeable but are
not — using `/r/` silently returns US-only-remote jobs.

### "Remote" is not a boolean on most platforms

- Wellfound has `remote: bool` **plus** `remoteConfig.kind`
  (`ONSITE_OR_REMOTE`, fully remote, hybrid…) and
  `accepted_remote_location_names`. `remoteConfig` is null in ~75% of hits.
- Correctness strategy: restrict region **server-side via the URL**, then
  take the platform's own `remote` flag as-is. Don't build client-side
  remote_ok/location lookup tables — they rot.
- Preserve the raw nuance fields in `<Name>JobDetail` even if unused today.

### Browser providers

- Prefer logged-in persistent tabs for sites behind auth (Upwork): tests
  must fail fast with `anyhow::bail!` if the tab is missing, never
  `eprintln!` + `Ok(vec![])`.
- JS regex inside `page.evaluate` differs from Rust — test in browser
  console first; scrape with `document.body.innerText`, not `textContent`
  (avoids script content).

### Rejection memory

Jobs rejected once (e.g. `non_english` language classification) are recorded
via `db.mark_rejected(platform, id, reason)` and auto-skipped on re-runs —
no re-classification cost.
