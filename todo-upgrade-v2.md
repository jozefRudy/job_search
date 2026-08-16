# SolidJS v2 Upgrade Plan

Status: **wait for stable v2** (current: `solid-js@2.0.0-rc.0`). Migration surface is small; do it once against stable APIs instead of chasing RC churn.

Solid v2 docs corpus: https://v2.solidjs.com/llms.txt (single-file: https://v2-rebuild--solid-docs-v2.netlify.app/llms-full.txt)

## FIRST CHECK when asked "can we upgrade now?"

**Check orval first — it is the biggest blocker and easiest to verify:**

Generated `src/generated/orval/jobsearch.ts` imports `mergeProps` from `solid-js` (removed in v2). Until orval updates its `solid-query` template for solid v2, generated code will not compile.

Check: `npm view orval version` and its changelog/templates for solid v2 (`merge` instead of `mergeProps`).

**If orval not ready: WAIT. Do not upgrade until orval fully supports solid v2.** No template patches, no client switch — orval readiness is the gate for the whole upgrade.

Then check remaining RCs stabilized: `solid-js@2`, `@solidjs/router@2`, `@tanstack/solid-query@6`, `vite-plugin-solid` v2-compatible major.

## Architecture confirmed

- Pure client-side SPA (Vite + `solidPlugin()`, `/api` proxy to Rust backend on :8080). No SSR, no SolidStart, none needed — private tool, no SEO concerns.

## Package bumps (when v2 stable)

| Package | Now | Target |
|---|---|---|
| `solid-js` | `^1.9.9` | `^2.0.0` |
| `@solidjs/web` (new pkg: client render, Portal/Dynamic split out) | — | `^2.0.0` |
| `@solidjs/router` | `^0.16.1` | `^2.0.0` (currently `2.0.0-next.16`, API carried over from 1.0) |
| `@tanstack/solid-query` | `^5.101.0` | `^6.0.0` (v6 required for solid v2; currently `6.0.0-rc.0`) — has its own v5→v6 breaking changes |
| `vite-plugin-solid` | `^2.11.8` | next major (v3+) |
| `@solidjs/testing-library` | `^0.8.10` | check v2 compat (used only in `src/api.test.tsx`) |

## Decision: stay on `@solidjs/router`

- Both routers have v2 tracks (`@tanstack/solid-router@2.0.0-rc.0` also exists).
- TanStack Router adds type-safe search params, loaders, file-based routing — none needed here (8 files, simple list/detail/dev routes).
- SSR (main TanStack Router selling point) irrelevant for this app.
- Switching = rewriting all routing to `createRouter`/`createRootRoute` API. Not worth it.

## What changed in solid v2 (verified during research)

- **Package split**: DOM rendering moved out of `solid-js` into new `@solidjs/web` (client render, hydration, SSR, `Portal`/`Dynamic`). `render` import becomes `import { render } from '@solidjs/web'`.
- **Core APIs partly changed** (verified against `2.0.0-rc.0` type exports):
  - `createResource` **removed** — async moved into signal pipeline: `refresh()`, `resolve()`, `latest()`, `onSettled()`, `isPending()`, `createLoadingBoundary` (Refreshable signals). Unused in our codebase — no work.
  - `createEffect` **signature change**: two-phase split `createEffect(compute, effectFn | { effect, error }, options?)` — compute tracks deps, effectFn runs untracked. Same for `createRenderEffect`. 1 usage to migrate.
  - `mergeProps` **removed** → `merge()` from `@solidjs/signals`. 10 usages.
  - `splitProps` **removed**. 14 usages to migrate.
  - `onMount` **removed**. 1 usage.
  - Unchanged: `createSignal`, `createMemo`, `createStore`, `onCleanup`, `untrack`, `Show`/`For`/`Switch`/`Match`, `lazy`, `children`, context APIs.
- Router/query ecosystems each need **their own major bump** (router 1→2, query 5→6) — plus the above core migrations.
- (Orval blocker — see FIRST CHECK section above.)

## Code surface (verified small)

- `solid-js` imported in 30 files. v2-stable: `createSignal` (9), `createMemo` (1), `Show` (7), `For` (4), `lazy` (1), `onCleanup` (1), `JSX`/`Component` types (18), single `render()` in `src/index.tsx` (import moves to `@solidjs/web`).
- **Needs migration**: `splitProps` (14), `mergeProps` (10), `createEffect` (1, new signature), `onMount` (1).
- No `solid-js/store`, no `createResource`, no Suspense edge cases.
- Orval-generated code (`src/generated/orval/jobsearch.ts`) — see FIRST CHECK section.
- Main friction: `@tanstack/solid-query` v5→v6 breaking changes in manual wrappers (`src/api.ts`).

## Migration mapping (from official v2 migration guide)

| v1 | v2 |
|---|---|
| `splitProps(props, ["a","b"])` | `omit(props, "a", "b")` — reactive proxy of "rest" only; handled keys read directly off `props` (don't destructure props, v2 dev-warns) |
| `mergeProps(defaults, props)` | `merge(defaults, props)` — note: `undefined` is now an explicit overriding value |
| `onMount(() => {...})` + `onCleanup` | `onSettled(() => { ...; return cleanup })` — runs once after first stable render, cleanup returned from callback, `onCleanup` not allowed inside |
| `createResource` | async `createMemo(() => fetch(...))` + `<Loading>` boundary; `refetch`→`refresh()`, `.loading`→`isPending()`, `.error`→`<Errored>` boundary, `.latest`→`latest()` |
| `on(deps, fn)` | deps in effect compute phase + `defer` option |
| `catchError`/`onError` | `<Errored>` boundary or effect `error` callback |
| `batch`, `createMutable`, `produce`, `createSelector`, `createDeferred`, `indexArray` | removed (default batching / draft setters / `createProjection` / external scheduling / `mapArray({keyed:false})`) |

## Upgrade checklist

1. Wait for `solid-js@2.0.0` stable **and** orval solid-query template with solid v2 support (hard gate, see FIRST CHECK).
2. Bump packages per table above.
3. `regen-api` (orval regen — verify no `mergeProps` import in output).
4. Fix `@tanstack/solid-query` v6 API changes in `src/api.ts`.
5. Validate: `cd frontend && pnpm typecheck && pnpm check && pnpm test run && pnpm build`.
6. E2E check: `cargo run` + `pnpm start`, verify key flows in browser.
