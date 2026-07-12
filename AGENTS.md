# Project Rules

## After Code Changes

Don't relax clippy rules -> #[allow(clippy::*)]
After completing code changes, run validation:
```bash
cargo build && cargo clippy --all-targets && cargo test && cargo fmt
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

## Documentation

- Check `target/md_docs/` when using unfamiliar APIs, 3rd party crates, or trait/method signature errors
- When writing code, don't add redundant comments. Only minimum if at all
- Write idiomatic Rust

## Regex in JS evaluate blocks

- JavaScript regex syntax differs from Rust — `+?` is valid in JS but can cause `Invalid regular expression flags` if combined with `/i` in certain ways. Test regex in browser console first.
- Use `document.body.innerText` instead of `textContent` when scraping to avoid getting script content mixed in.

## Browser Integration Tests with Live Browser

- **For sites requiring login / slow load (Upwork):** do NOT automate tab creation in test. Manual setup: open tab, login, then run test. Test fails fast with clear error if tab absent.
- Prefer `anyhow::bail!("message")` over `eprintln!` + `Ok(vec![])` when precondition fails — fail fast surfaces real errors in tests.
- Never use `unwrap` in tests — use `.expect("msg")` or `?`.
- Fewer comprehensive tests beat many trivial ones.

## Frontend

After frontend changes, run:
```bash
cd frontend && pnpm typecheck && pnpm check && pnpm test run && pnpm build
```
- **SolidJS reactivity** — derived values must be functions or inline in JSX. Const assignments stale after first render.
- **Design system** — reuse primitives in `src/components/ui/` before raw Daisy/Tailwind.
- **Pattern matching** — prefer `ts-pattern` exhaustive matching over `if/else` chains and `switch`.
- **E2E check** — for all UI changes, run backend (`cargo run`) + frontend (`pnpm start`) together, verify key flows in browser. Frontend proxies `/api` to `localhost:8080`.

## API Generation (OpenAPI + Orval)

- **Backend** — `utoipa` + `utoipa-axum` on Rust handlers. `#[derive(ToSchema)]` on models, `#[utoipa::path(...)]` on handlers, `OpenApiRouter` for route collection.
- **Frontend** — `orval` with `client: 'fetch'` generates typed fetch functions + schemas from `/api/openapi.json`.
- **TanStack wrappers** — manual thin wrappers in `api.ts` using `@tanstack/solid-query`. Orval's `solid-query` client is broken for v5 (uses removed `SolidMutationOptions` type).
- **Regenerate** — `regen-api` script starts backend, waits for `/api/openapi.json`, runs `pnpm orval`. Commit generated files to version control.
