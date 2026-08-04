// Fetch one SSR page and extract jobs from embedded Apollo state.
// config = { "url": pageUrl }
//
// Steps:
//   1. html = await (await fetch(config.url, { credentials: "include" })).text()
//      — return { error: `http ${resp.status}` } on !resp.ok
//   2. Locate marker '"apolloState":{"data":{' in html; if absent return
//      { error: "no apolloState — not logged in or markup changed" }
//   3. Brace-match from the '{' after '"data":' to extract the data object;
//      JSON.parse it (keys: "JobListingSearchResult:<id>", "StartupResult:<id>", ...)
//   4. Build jobId→startup map: for each StartupResult entry, for each ref in
//      highlightedJobListings, map ref id → startup (two passes — startup may
//      appear after the job in key order; one startup can highlight 2+ jobs)
//   5. For each JobListingSearchResult entry:
//      - look up startup via reverse map
//      - flatten remoteConfig?.kind -> remote_kind (null when remoteConfig absent,
//        ~75% of jobs)
//      - emit camelCase->snake_case mapped object matching RawWellfoundJob
//   6. Return { jobs: [...] } (order = object key insertion order, fine)
//
// Note: JS regex caveats per AGENTS.md; brace matching avoids regex on JSON.
(async function (config) {
  // TODO Phase 3: implement steps above
  return { error: "TODO Phase 3" };
})(__CONFIG__)
