//! Reddit platform client.
//!
//! Fetches jobs from configured subreddit sources via the user's browser
//! session (anonymous `.json` requests get 403). Two source kinds:
//! - `Posts`: subreddit `/new.json`, keep titles starting `[HIRING]` (r/jobbit)
//! - `Megathread`: latest "Who's Hiring" thread, top-level comments = jobs (r/rust)
//!
//! TODO(phase2): implement scraper mirroring linkedin.rs structure.

// TODO(phase2): const FETCH_JS: &str = include_str!("reddit/fetch.js");
// TODO(phase2): const JS_REQUEST_PLACEHOLDER: &str = "__REDDIT_REQUEST__";

// TODO(phase2): wire types
// use crate::browser::BrowserExt;
// use crate::db::{Db, UpsertResult};
// use crate::extractors::llm::LlmExtractor;
// use crate::extractors::llm_reddit;
// use crate::models::{Data, Job, Platform, Rating, RedditJobDetail};
// use crate::platforms::{FetchState, PlatformClient};

// TODO(phase2): #[derive(Deserialize)] struct RedditRequest { url: String }
//   Serialized into JS; JS returns RedditResponse { status: u16, body: Option<serde_json::Value> }
//   so Rust can implement 429 backoff.

// TODO(phase2): hardcoded sources (like HN THREAD_QUERY const — thread naming
//   is implicit knowledge, not user config). New subreddit = code edit.
// enum SourceKind { Posts, Megathread }
// struct Source { subreddit: &'static str, kind: SourceKind,
//   query: Option<&'static str>, title_filter: Option<&'static str> }
// const SOURCES: &[Source] = &[
//   jobbit / Posts        // keep [HIRING] titles only
//   rust   / Megathread { query: "Who's Hiring", title_filter: "(?i)who's hiring" }
// ];

// TODO(phase2): pub struct RedditScraper {
//   jobbit_extractor: LlmExtractor<llm_reddit::JobbitFields>,
//   megathread_extractor: LlmExtractor<llm_reddit::MegathreadFields>,
// }
//   // two extractors picked by source kind; verify() both before processing
// impl RedditScraper:
//   pub fn new(llm_cli: &str, location: &str) -> Result<Self>
//   async fn ensure_reddit_tab(&self, browser: &Browser) -> Result<()>
//     // bail!("Reddit login required...") handled in JS; here only tab presence like linkedin
//   async fn fetch_json(&self, page: &Page, url: &str) -> Result<serde_json::Value>
//     // page.evaluate(FETCH_JS with request); retry with exponential backoff on 429 (max 3)
//   async fn fetch_source(&self, page: &Page, db: &Db, source: &Source, pause_ms: u64, state: &mut FetchState) -> Result<()>
//     // match source.kind { Posts => fetch_posts_source, Megathread => fetch_megathread_source }
//   async fn fetch_posts_source(...)   // GET /r/{sub}/new.json?limit=100; filter [HIRING]; external_id = t3_ name
//   async fn fetch_megathread_source(...) // GET /r/{sub}/search.json?q=<query>&restrict_sr=1&sort=top&t=all&limit=25
//     // (sort=new is broken, ignores q — verified); filter title regex,
//     // take thread with max created_utc; GET /comments/{id}.json?limit=500&depth=1;
//     // top-level t1 children only; external_id = t1_ name; skip [deleted]/[removed],
//     // mod comments rejected by LLM is_job_ad
//   async fn build_job(&self, subreddit: &str, external_id: &str, title: &str, body: &str,
//     author: &str, permalink: &str, created_utc: f64) -> Result<Option<Job>>
//     // llm extract; !is_job_ad => None; fill remote/budget/tags/company/role/location;
//     // title = role or truncated first line (200 chars); url = https://www.reddit.com{permalink}
//   async fn process_candidate(&self, db, page, state, candidate..., pause_ms) -> Result<()>
//     // db.filter_new gate, db.mark_rejected on not_job_ad/parse_failed, db.upsert_job,
//     // progress_line, sleep(pause_ms) between LLM calls

// TODO(phase2): impl PlatformClient for RedditScraper
//   fn name(&self) -> &'static str { "reddit" }
//   async fn fetch_with_browser(&self, browser: &Browser, db: &Db, _url: &str, pause_ms: u64) -> Result<FetchState>
//     // ensure_reddit_tab; open page on https://www.reddit.com; loop sources -> fetch_source; close page

// TODO(phase3): unit tests in `mod tests` (pure, no LLM) — mirror hackernews.rs tests:
//   test_parse_posts_listing_fixture — reddit/fixtures/jobbit_new.json; yields only [HIRING] posts,
//     filter case-insensitive, external_id = t3_ name, [FOR HIRE] excluded
//   test_parse_search_fixture — reddit/fixtures/rust_search.json; title regex filter,
//     picks thread with max created_utc
//   test_parse_comments_fixture — reddit/fixtures/rust_comments.json; top-level t1 only,
//     skips [deleted]/[removed], skips AutoModerator
//   test_title_truncation — role fallback truncates first line at 200 chars with ellipsis
// TODO(phase3): integration test in tests/browser_integration.rs — mirror linkedin tests:
//   #[ignore = "requires Chromium browser with CDP and reddit.com tab open"]
//   test_reddit_fetch_jobbit — with_browser; fetch /r/jobbit/new.json via FETCH_JS,
//     assert [HIRING] posts parsed, external_ids start with t3_
//   NOTE: manual tab setup per AGENTS.md (open reddit tab before running); fail fast
//     with anyhow::bail! if tab absent
