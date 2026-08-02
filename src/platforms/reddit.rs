//! Reddit platform client.
//!
//! Fetches jobs from configured subreddit sources via the user's browser
//! session (anonymous `.json` requests get 403). Two source kinds:
//! - `Posts`: subreddit `/new.json`, keep titles starting `[HIRING]` (r/jobbit)
//! - `Megathread`: latest "Who's Hiring" thread, top-level comments = jobs (r/rust)

use crate::browser::BrowserExt;
use crate::db::Db;
use crate::extractors::llm::LlmExtractor;
use crate::extractors::llm_reddit;
use crate::models::{Job, Platform};
use crate::platforms::{FetchState, PlatformClient};
use anyhow::Result;
use chromiumoxide::browser::Browser;
use chromiumoxide::page::Page;
use serde::Deserialize;

const FETCH_JS: &str = include_str!("reddit/fetch.js");
const JS_REQUEST_PLACEHOLDER: &str = "__REDDIT_REQUEST__";

#[derive(Deserialize)]
struct RedditRequest {
    url: String,
}

/// Hardcoded sources (like HN THREAD_QUERY const — thread naming is implicit
/// knowledge, not user config). New subreddit = code edit.
/// Each variant carries subreddit + only the data its fetch strategy needs.
#[derive(Debug, Clone, Copy)]
enum Source {
    /// Subreddit `/new.json` listing; keep posts whose title starts with
    /// `title_filter` (case-insensitive).
    Posts {
        subreddit: &'static str,
        title_filter: &'static str,
    },
    /// Latest thread matching `query` + `title_filter` regex;
    /// top-level comments = jobs.
    Megathread {
        subreddit: &'static str,
        query: &'static str,
        title_filter: &'static str,
    },
}

impl Source {
    fn subreddit(&self) -> &'static str {
        match self {
            Source::Posts { subreddit, .. } | Source::Megathread { subreddit, .. } => subreddit,
        }
    }
}

const SOURCES: &[Source] = &[
    Source::Posts {
        subreddit: "jobbit",
        title_filter: "[hiring]",
    },
    Source::Megathread {
        subreddit: "rust",
        query: "Who's Hiring",
        title_filter: "(?i)who's hiring",
    },
];

pub struct RedditScraper {
    jobbit_extractor: LlmExtractor<llm_reddit::JobbitFields>,
    megathread_extractor: LlmExtractor<llm_reddit::MegathreadFields>,
}

impl RedditScraper {
    pub fn new(llm_cli: &str, location: &str) -> Result<Self> {
        todo!("phase3: build both extractors via LlmExtractor::from_cli + with_prompt_context")
    }

    async fn ensure_reddit_tab(&self, browser: &Browser) -> Result<()> {
        todo!("phase3: bail!(\"Reddit requires open reddit.com tab in browser\") if no reddit.com tab, mirroring LinkedInScraper::ensure_linkedin_tab")
    }

    /// GET `url` via FETCH_JS in the reddit tab; retry with exponential
    /// backoff on 429 (max 3 attempts).
    async fn fetch_json(&self, page: &Page, url: &str) -> Result<serde_json::Value> {
        todo!("phase3: FETCH_JS.replace(JS_REQUEST_PLACEHOLDER, request json); RedditResponse {{ status, body }}; backoff on 429")
    }

    async fn fetch_source(
        &self,
        page: &Page,
        db: &Db,
        source: &Source,
        pause_ms: u64,
        state: &mut FetchState,
    ) -> Result<()> {
        todo!("phase3: match source {{ Posts {{..}} => fetch_posts_source, Megathread {{..}} => fetch_megathread_source }}")
    }

    /// GET /r/{sub}/new.json?limit=100; filter [HIRING]; external_id = t3_ name.
    async fn fetch_posts_source(
        &self,
        page: &Page,
        db: &Db,
        source: &Source,
        pause_ms: u64,
        state: &mut FetchState,
    ) -> Result<()> {
        todo!("phase3")
    }

    /// GET /r/{sub}/search.json?q=<query>&restrict_sr=1&sort=top&t=all&limit=25
    /// (sort=new is broken, ignores q — verified); filter title regex, take
    /// thread with max created_utc; GET /comments/{id}.json?limit=500&depth=1;
    /// top-level t1 children only; external_id = t1_ name;
    /// skip [deleted]/[removed]; mod comments rejected by LLM is_job_ad.
    async fn fetch_megathread_source(
        &self,
        page: &Page,
        db: &Db,
        source: &Source,
        pause_ms: u64,
        state: &mut FetchState,
    ) -> Result<()> {
        todo!("phase3")
    }

    /// LLM extract; !is_job_ad => None; fill remote/budget/tags/company/role/
    /// location; title = role or truncated first line (200 chars);
    /// url = https://www.reddit.com{permalink}.
    async fn build_job(
        &self,
        subreddit: &str,
        external_id: &str,
        title: &str,
        body: &str,
        author: &str,
        permalink: &str,
        created_utc: f64,
    ) -> Result<Option<Job>> {
        todo!("phase3")
    }

    /// db.filter_new gate, db.mark_rejected on not_job_ad/parse_failed,
    /// db.upsert_job, progress_line, sleep(pause_ms) between LLM calls.
    async fn process_candidate(
        &self,
        db: &Db,
        page: &Page,
        state: &mut FetchState,
        candidate: &str,
        pause_ms: u64,
    ) -> Result<()> {
        todo!("phase3: candidate param placeholder — refine signature with actual candidate struct")
    }
}

#[async_trait::async_trait]
impl PlatformClient for RedditScraper {
    fn name(&self) -> &'static str {
        "reddit"
    }

    async fn fetch_with_browser(
        &self,
        browser: &Browser,
        db: &Db,
        _url: &str,
        pause_ms: u64,
    ) -> Result<FetchState> {
        todo!("phase3: ensure_reddit_tab; open page on https://www.reddit.com; verify() both extractors; loop SOURCES -> fetch_source; close page")
    }
}

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
