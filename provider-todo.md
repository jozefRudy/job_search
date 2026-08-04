# TODO: Wellfound integration

New provider `wellfound`, modeled on `workatastartup` (Phase 1-3 workflow per AGENTS.md).

## Verified facts (this session)

- Wellfound volume: **1828 remote SWE jobs** (45 pages) vs WaaS 163 eng+remote — ~11x breadth
- Logged-in session required; user has account
- Site is GraphQL-based (needs capture before stubbing — do NOT assume endpoints)
- Pagination cursor-based (verify during capture, don't assume page param)

## Steps

### 0. Recon (before Phase 1)
- [ ] Capture network on https://wellfound.com/role/r/software-engineer (`browser_capture_network`, `wait_for: "graphql"`) — get endpoint, query shape, variables, headers
- [ ] Identify cursor/pagination mechanism from response
- [ ] Identify hit fields (title, company, description, salary, remote, posted date)

### Phase 1-3 (mirror workatastartup)
- [ ] `src/platforms/wellfound.rs` + `src/platforms/wellfound/*.js` — filters-from-URL struct, raw hit struct, scraper, `PlatformClient`
- [ ] `src/models.rs`: `WellfoundJobDetail`, `Data::Wellfound`, `Platform::Wellfound` + ALL match arms (Display, FromStr, company(), searchable-text)
- [ ] `src/db.rs` + `src/embeddings_store.rs`: default-construction arms
- [ ] `src/cli.rs` (update + list variants), `src/config.rs` (`providers.wellfound.urls` + sample), `src/main.rs` (wiring)
- [ ] Unit tests: URL parse, filter mapping, hit→Job from recorded JSON sample
- [ ] Integration test in `tests/browser_integration.rs` (`#[ignore]`, `with_browser(60, ...)`) — run FIRST against live session before trusting field types
- [ ] Frontend: `regen-api`, `JobList.tsx` label `Wellfound` + sort map, `JobDetail.tsx` detail card
- [ ] README provider link entry

## Lessons from workatastartup session (apply here)

1. **Live integration test catches what unit tests can't.** Algolia hits had `min_experience: null` and `has_*: null` — only surfaced on live fetch. Write integration test early, run before finalizing field types.
2. **`#[serde(default)]` does NOT cover explicit JSON `null`** — use `Option<T>` for any field that can be null; don't "fix at boundary" with `unwrap_or(0)` (conflates "new grads ok" with "unknown"). Keep `Option` end-to-end.
3. **Session keys/tokens: extract inside page context**, e.g. from `performance.getEntriesByType('resource')`; they expire per session — never hardcode. Retry-with-wait for the key to appear after tab navigation (×20 × 500ms pattern).
4. **No early-stop pagination heuristics** when full scan is cheap (hits are complete, upsert idempotent). Simpler = more robust.
5. **New Platform variant = exhaustive match arms in 6+ places** (models Data/company/searchable-text/Display/FromStr, db.rs, embeddings_store.rs) — compiler finds them, but check `grep "Data::Reddit"` style sites proactively.
6. **Watch clippy `too_many_lines` (100)** when adding match arms to long fns — extract shared arm bodies into a helper (see `list_common` in main.rs) instead of copy-pasting the 7th arm.
7. **edit tool: small unique oldText** — a sloppy multi-line oldText deleted `fetch_and_store`'s signature in Phase 2. Build after every edit batch.
8. **JS regex/scraping**: use `document.body.innerText`, test regex in console first (AGENTS.md).
9. **E2E verification of UI** needs backend+frontend running; if browser tooling is stuck, verify via `curl localhost:3000/src/components/X.tsx` module transform.
10. **Validation gates**: `cargo build && cargo clippy --all-targets && cargo test && cargo fmt`; frontend `pnpm typecheck && pnpm check && pnpm test run && pnpm build`; `regen-api` after Platform enum change.
11. **Don't copy a broken assumption from recon**: check actual pagination from captured response, not from URL params guess.
