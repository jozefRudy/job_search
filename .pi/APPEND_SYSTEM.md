# ChainSignals Project Rules

## After Code Changes

After completing code changes, run validation:
```bash
cd backend && cargo build && cargo clippy -- -D warnings && cargo test && cargo fmt
```

Integration tests (live RPC, >1min) — takes 1 minute, run after changes related to api clients

```bash
cd backend && cargo test -- --include-ignored
```

## SQLx

- **Static queries** — always use `query_as!` macro for compile-time checking.
- **Dynamic queries** (conditional WHERE/LIMIT) — manual `sqlx::query_as::<_, Row>(&query_string)` with `.bind()` is OK. But prefer static queries when possible.
- `SQLX_OFFLINE = "true"` is set as env var
- Run `cargo-sqlx-prepare` (not `cargo sqlx prepare` — former prepares db) after any query change to update `.sqlx/`
- `devenv` script prepares everything then runs `cargo sqlx prepare`

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

## Session Efficiency

- If user says "don't use X", stop immediately — don't keep trying. (e.g. `browser_navigate` after user said use rust code)
- Read docs **before** trying 3 variants of a feature. (e.g. checked md_docs for window state only after trying hidden/minimized/offscreen)
- `grep` for all field usages before renaming — prevents dead references.
- When adding CLI commands, update both `cli.rs` enum AND `main.rs` match arm.

## Architecture Reference

- `PATTERNS.md` — LanceDB, Postgres, Auth, API, Frontend patterns with code examples. Read when building new feature.
- `PLAN.md` — Current project roadmap and priorities. Read before starting new work.
- `README.md` — Project overview, pricing, competition. Read once for context.
