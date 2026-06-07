# ChainSignals Project Rules

## After Code Changes

After completing code changes, run validation:
```bash
cargo build && cargo clippy -- -D warnings && cargo test && cargo fmt
```

Integration tests, run after changes related to api clients

```bash
cargo test -- --include-ignored
```

## SQLx

- **Static queries** — always use `query!` / `query_as!` macro for compile-time checking.
- **Optional filters** — use `WHERE (?1 IS NULL OR platform = ?1)` pattern. No dynamic SQL needed.
- **SQLite timestamps** — macro infers `NaiveDateTime`. Use `NaiveDateTime` in row structs, convert to `DateTime<Utc>` in `From` impl.
- **Schema alignment** — macro checks `NOT NULL` vs nullable. Mismatch = compile error. Fix schema or row struct types.
- `SQLX_OFFLINE = "true"` is set as env var
- `cargo sqlx prepare` — devenv `enterShell` auto-runs on shell entry. For active editing, run `direnv reload` or `cargo sqlx prepare` manually.
- Commit `.sqlx/` to version control — enables `SQLX_OFFLINE=true` builds without live DB.
- Migrations in `migrations/` — `sqlx::migrate!("./migrations").run(&pool)` on startup.

## Design Rules

1. **Start simple.** Add complexity only when proven needed.
   - Bad: `Vec<(T, Option<usize>)>` for pairing → Good: `counterparty: Option<String>`
   - Bad: `Arc<RefCell<...>>` for cross-references → Good: clone the string

2. **Use existing abstractions.** Don't reimplement parsing.
   - Bad: Manual event iteration + base64 decode in test
   - Good: Use standard `SeiParser.parse_block_signals()`
   - If parser exists, use it. Don't duplicate logic.

3. **Read before editing.** Understand the full context.
   - Bad: Rename fields without checking all usages
   - Good: `grep` for all references first

4. **Types and immutability first.** When data doesn't fit your type, fix it at the boundary (deserializers, `From`/`TryFrom`) — never with manual mutation or pre-processing.
   - Bad: Mutate `serde_json::Value` before deserializing; chain `.strip_prefix("posted ").strip_suffix(" ago")` for every text variant
   - Good: `#[serde(deserialize_with)]`; single regex that captures the pattern regardless of surrounding text
   - Rule: Reaching for `mut` or chained string transforms to shape data = missed abstraction

5. **Invalid states unrepresentable.** If runtime checks (`bail!`, `if` guards) validate argument combinations, the type system wasn't used.
   - Bad: Flat `--sort` flag with `--platform`, runtime `bail!("--sort viewed only for upwork")`
   - Good: Clap subcommands — `list upwork --sort viewed`, `list nofluff` — platform-specific args live only where valid
   - Same for `let-else` / `unreachable!` in caller: push the invariant into the data model (`Job::upwork()` method) so misuse panics at compile time or in one central place

## Imports

Use `use` for repeated paths. No fully-qualified repetition.
- Bad: `tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;`
- Good: `use tokio::time::{sleep, Duration};` then `sleep(Duration::from_secs(2)).await;`

## Testing

- Use `?` in async tests — return `Result<()>` instead of `unwrap`. Cleaner error messages, no panic backtraces.
- **Temp files** — use `tempfile` crate. Auto-deletes on drop, even on panic. Never manual `remove_file` in tests.
- Run **full** `cargo test` (not `--lib`) before claiming done — integration tests matter
- When debugging distribution output, check sample count vs bin count (need N >= bins for quantile display)
- For parser base64: real RPC data is base64 encoded, test fixtures use plain text. Handle both.

## Infrastructure

- Free RPC tiers first. Check rate limits before assuming paid node needed.
- 100 req/s free tier = way more than 12 blocks/sec (ETH). Single machine handles all users.
- Flat pricing correct for push architecture. Marginal cost per user ≈ $0.

## Documentation

- Check `target/md_docs/` when using unfamiliar APIs, 3rd party crates, or trait/method signature errors
- When writing code, don't add redundant comments. Only minimum if at all
- Write idiomatic Rust

## Validation Strategy

- When faced with uncertainty, prefer building spike to validate before committing architecture
- Risky assumptions first, defer low-uncertainty work

## Browser Automation (chromiumoxide)

- **Connect to existing browser first** — `Browser::connect("http://localhost:9222")`. Gets user's cookies/session, no focus stealing. Fallback to `Browser::launch()` only if not running.
- **Background tabs** — use `CreateTargetParams::builder().url("about:blank").background(true)`, then `page.goto(url)`. `CreateTargetParams::url(some_url)` with `background(true)` does not eagerly load in background.
- **`GetTargetsParams` returns ALL targets** — service workers, extensions, etc. Always filter `t.r#type == "page"` before matching.
- **`/json/new` HTTP endpoint creates foreground tabs** — has no background param. Use CDP `Target.createTarget` via `Browser::new_page(CreateTargetParams)` instead.
- **`open -g` on macOS** works for launching fresh app instance in background. Sending URLs to already-running apps via `open` can still activate. Use CDP tab creation for already-running browsers.
- **Separate concerns** — `BrowserManager` manages connection lifecycle (connect/launch/ensure). Tab operations belong on `Browser` via extension trait (`BrowserExt`), not on manager.
- **Use `page.wait_for_navigation()`** after `goto()` instead of blind `sleep`. Still need polling loops for JS-rendered SPA content.
- **New tabs inherit cookies** from existing browser session. No need to control existing tabs directly (chromiumoxide can't reliably get handles for pre-existing tabs).
- **Check `target/md_docs/` for API signatures** before guessing — e.g. `BrowserConfigBuilder` methods, CDP command types.

## Regex in JS evaluate blocks

- JavaScript regex syntax differs from Rust — `+?` is valid in JS but can cause `Invalid regular expression flags` if combined with `/i` in certain ways. Test regex in browser console first.
- Use `document.body.innerText` instead of `textContent` when scraping to avoid getting script content mixed in.

## Browser Integration Tests with Live Browser

- **For sites requiring login / slow load (Upwork):** do NOT automate tab creation in test. Manual setup: open tab, login, then run test. Test fails fast with clear error if tab absent.
- Prefer `anyhow::bail!("message")` over `eprintln!` + `Ok(vec![])` when precondition fails — fail fast surfaces real errors in tests.
- Never use `unwrap` in tests — use `.expect("msg")` or `?`.
- Fewer comprehensive tests beat many trivial ones.

## Refactoring

- **Avoid sed for code transformations.** sed corrupts imports, breaks syntax, and produces unhelpful errors. Use `write` for full-file rewrites or `edit` with exact text matches.
- **Avoid scripted bulk rewrites across multiple files.** Python string replacement, bash heredocs, and sed silently corrupt escaping, formatting, and macro syntax. Enumerate exact per-file changes first, then apply with targeted `edit`. Full-file rewrites only for small, fully-controlled files.
- **Plan refactor scope before touching files.** Cascading type changes across 5+ files cause compile-error whack-a-mole. Map all affected files first (models, db, platform modules, main.rs, tests).
- **Prefer `#[allow(...)]` over boxing enum variants.** `Box<T>` adds indirection and noise. For large enum variants that are rarely cloned, suppress the lint instead.

## Browser Scraping JS

- **Extract JS snippets to `.js` files, use `include_str!` directly in scraper module.** Create `src/platforms/<platform>/snippet.js` files, load with `const SNIPPET_JS: &str = include_str!("<platform>/snippet.js");` in the scraper `.rs` file. No thin `*_js.rs` loader modules needed.
- **Avoid framework-internal state.** `window.__NUXT__`, React props, hydrated globals break silently on site updates. Prefer stable DOM selectors or visible text (`document.body.innerText`).
- **Simple fallback chain:** DOM selector first → regex on `innerText` fallback → empty string default. No `try/catch` around optional chaining.
- **Simplify with small JS helpers.** A 2-line `rx(pattern)` or `liText(selector)` helper removes repetitive `match`/`?.trim()` boilerplate.

## Session Efficiency

- If user says "don't use X", stop immediately — don't keep trying. (e.g. `browser_navigate` after user said use rust code)
- Read docs **before** trying 3 variants of a feature. (e.g. checked md_docs for window state only after trying hidden/minimized/offscreen)
- `grep` for all field usages before renaming — prevents dead references.
- **Changing public function signatures:** `grep` all call sites including tests and ignored integration tests before editing. Saves compile-error whack-a-mole.
- When adding CLI commands, update both `cli.rs` enum AND `main.rs` match arm.

## CLI Design

- **Platform-specific options = subcommands, not flags.** When different platforms need different args (`--tier` for Upwork, `--employment` for NoFluffJobs), use clap subcommands (`update upwork`, `update nofluff`) instead of flat flags with `--platform`. Clean, self-documenting, no ambiguity.
- **Generic traits stay generic.** Never put platform-specific types (`UpworkTier`, `hourly_rate_min`) into a generic `PlatformClient` trait. Store config in scraper struct fields instead.

## Architecture Reference

- `PATTERNS.md` — LanceDB, Postgres, Auth, API, Frontend patterns with code examples. Read when building new feature.
- `PLAN.md` — Current project roadmap and priorities. Read before starting new work.
- `README.md` — Project overview, pricing, competition. Read once for context.

