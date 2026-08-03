//! Reddit platform client.
//!
//! Fetches jobs via the user's browser session (anonymous `.json` requests
//! get 403). Source kind: latest "Who's Hiring" megathread, top-level
//! comments = jobs (r/rust).

use crate::browser::BrowserExt;
use crate::db::{Db, UpsertResult};
use crate::extractors::llm::LlmExtractor;
use crate::extractors::llm_reddit::RustFields;
use crate::models::{Data, Job, Platform, Rating, RedditJobDetail};
use crate::platforms::{FetchState, PlatformClient, truncate_with_ellipsis};
use crate::term::CursorGuard;
use anyhow::{Context, Result, bail};
use chromiumoxide::browser::Browser;
use chromiumoxide::page::Page;
use chrono::{DateTime, Utc};
use owo_colors::OwoColorize;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;

const FETCH_JS: &str = include_str!("reddit/fetch.js");
const JS_REQUEST_PLACEHOLDER: &str = "__REDDIT_REQUEST__";
const MAX_TITLE_LEN: usize = 200;
const MAX_RETRIES: u32 = 3;

/// Hardcoded sources (like HN THREAD_QUERY const — thread naming is implicit
/// knowledge, not user config). New subreddit = code edit.
/// Latest thread matching `query` + `title_filter` regex; top-level
/// comments = jobs.
#[derive(Debug, Clone, Copy)]
pub struct Source {
    pub subreddit: &'static str,
    pub query: &'static str,
    pub title_filter: &'static str,
}

const SOURCES: &[Source] = &[Source {
    subreddit: "rust",
    query: "Who's Hiring",
    title_filter: r"(?i)who's hiring",
}];

#[derive(Serialize)]
struct RedditRequest {
    url: String,
}

#[derive(Deserialize)]
struct RedditResponse {
    status: u16,
    body: Option<serde_json::Value>,
}

/// Normalized job candidate parsed from a megathread comment.
#[derive(Debug, Clone, Serialize)]
pub struct Candidate {
    pub external_id: String,
    pub title: String,
    pub body: String,
    pub author: String,
    pub permalink: String,
    pub created_utc: f64,
}

pub struct RedditScraper {
    rust_extractor: LlmExtractor<RustFields>,
}

impl RedditScraper {
    pub fn new(llm_bin: &str, location: &str) -> Result<Self> {
        let context = format!("Candidate location: {location}");
        Ok(Self {
            rust_extractor: LlmExtractor::<RustFields>::from_bin(llm_bin)
                .with_prompt_context(context),
        })
    }

    async fn ensure_reddit_tab(&self, browser: &Browser) -> Result<()> {
        let hosts: Vec<_> = browser
            .get_page_urls()
            .await?
            .into_iter()
            .filter_map(|u| crate::browser::host_of(&u))
            .collect();
        if !hosts.iter().any(|h| h.contains("reddit.com")) {
            bail!("Reddit requires open reddit.com tab in browser");
        }
        Ok(())
    }

    /// GET `url` via FETCH_JS in the reddit tab; retry with exponential
    /// backoff on 429 (max 3 attempts).
    pub async fn fetch_json(&self, page: &Page, url: &str) -> Result<serde_json::Value> {
        let request = serde_json::to_string(&RedditRequest {
            url: url.to_string(),
        })?;
        let js = FETCH_JS.replace(JS_REQUEST_PLACEHOLDER, &request);

        for attempt in 0..MAX_RETRIES {
            let response: RedditResponse = page
                .evaluate(js.as_str())
                .await
                .context("reddit fetch js failed")?
                .into_value()
                .context("reddit fetch js returned unexpected shape")?;
            match response.status {
                429 => {
                    let wait = Duration::from_millis(1000 * 2_i64.pow(attempt) as u64);
                    eprintln!("    Reddit 429, retrying in {}s", wait.as_secs());
                    tokio::time::sleep(wait).await;
                }
                200 => {
                    return response
                        .body
                        .context("reddit fetch returned 200 without body");
                }
                status => bail!("reddit fetch failed with status {status} for {url}"),
            }
        }
        bail!("reddit fetch still rate-limited after {MAX_RETRIES} attempts for {url}")
    }

    /// Pick the thread id with max created_utc among titles matching
    /// `title_filter` regex from a `search.json` response.
    #[must_use]
    pub fn pick_megathread(json: &serde_json::Value, title_filter: &Regex) -> Option<String> {
        json.pointer("/data/children")?
            .as_array()?
            .iter()
            .filter_map(|c| {
                let d = c.get("data")?;
                let title = d.get("title")?.as_str()?;
                if !title_filter.is_match(title) {
                    return None;
                }
                Some((d.get("id")?.as_str()?, d.get("created_utc")?.as_f64()?))
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(id, _)| id.to_string())
    }

    /// Parse `/comments/{id}.json` response ([post, comments] array);
    /// top-level t1 children only; skips [deleted]/[removed] and AutoModerator.
    #[must_use]
    pub fn parse_comments(json: &serde_json::Value) -> Vec<Candidate> {
        json.get(1)
            .and_then(|c| c.pointer("/data/children"))
            .and_then(serde_json::Value::as_array)
            .map(|children| {
                children
                    .iter()
                    .filter(|c| c.get("kind").and_then(|k| k.as_str()) == Some("t1"))
                    .filter_map(|c| {
                        let d = c.get("data")?;
                        let body = d.get("body")?.as_str()?;
                        let author = d.get("author")?.as_str()?;
                        if body == "[deleted]" || body == "[removed]" || author == "AutoModerator" {
                            return None;
                        }
                        Some(Candidate {
                            external_id: d.get("name")?.as_str()?.to_string(),
                            title: body
                                .lines()
                                .find(|l| !l.trim().is_empty())
                                .map(str::trim)
                                .unwrap_or_default()
                                .to_string(),
                            body: body.to_string(),
                            author: author.to_string(),
                            permalink: d.get("permalink")?.as_str()?.to_string(),
                            created_utc: d.get("created_utc")?.as_f64()?,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    async fn fetch_source(
        &self,
        page: &Page,
        db: &Db,
        source: &Source,
        pause_ms: u64,
        state: &mut FetchState,
    ) -> Result<()> {
        let url = format!(
            "https://www.reddit.com/r/{}/search.json?q={}&restrict_sr=1&sort=top&t=all&limit=25",
            source.subreddit,
            urlencoding::encode(source.query)
        );
        let json = self.fetch_json(page, &url).await?;
        let re = Regex::new(source.title_filter)?;
        let thread_id = Self::pick_megathread(&json, &re)
            .context("no megathread found matching title filter")?;
        let url =
            format!("https://www.reddit.com/comments/{thread_id}.json?limit=500&depth=1&sort=new");
        let json = self.fetch_json(page, &url).await?;
        let candidates = Self::parse_comments(&json);

        let ids: Vec<String> = candidates.iter().map(|c| c.external_id.clone()).collect();
        let new_ids: HashSet<String> = db
            .filter_new(&Platform::Reddit, &ids)
            .await?
            .into_iter()
            .collect();
        state.inc_existing_n(ids.len().saturating_sub(new_ids.len()));

        let new_candidates: Vec<_> = candidates
            .into_iter()
            .filter(|c| new_ids.contains(&c.external_id))
            .collect();
        if new_candidates.is_empty() {
            return Ok(());
        }

        let total = new_candidates.len();
        for (idx, candidate) in new_candidates.into_iter().enumerate() {
            self.process_candidate(db, source, candidate, pause_ms, state, (idx + 1, total))
                .await?;
        }
        // dangling \r progress line is closed by CursorGuard in fetch_with_browser
        Ok(())
    }

    /// LLM extract; !is_job_ad => None; fill remote/budget/tags/company/role/
    /// location; title = role or truncated first line (200 chars);
    /// url = https://www.reddit.com{permalink}.
    async fn build_job(&self, source: &Source, candidate: &Candidate) -> Result<Option<Job>> {
        let fields = self.rust_extractor.extract(&candidate.body).await?;
        if !fields.is_job_ad {
            return Ok(None);
        }

        let company = fields.company.filter(|s| !s.is_empty());
        let role = fields.role.filter(|s| !s.is_empty());
        let location = fields.location.filter(|s| !s.is_empty());

        let title = role.clone().unwrap_or_else(|| candidate.title.clone());
        let title = truncate_with_ellipsis(&title, MAX_TITLE_LEN);

        let posted_at =
            DateTime::from_timestamp(candidate.created_utc as i64, 0).unwrap_or_else(Utc::now);

        Ok(Some(Job {
            id: 0,
            platform: Platform::Reddit,
            external_id: candidate.external_id.clone(),
            title,
            url: format!("https://www.reddit.com{}", candidate.permalink),
            budget: fields.budget,
            tags: fields.tags,
            raw: Data::Reddit {
                detail: RedditJobDetail {
                    subreddit: source.subreddit.to_string(),
                    author: candidate.author.clone(),
                    permalink: candidate.permalink.clone(),
                    company: company.clone(),
                    role,
                    location,
                    description: candidate.body.clone(),
                    posted_at,
                },
            },
            company,
            created_at: posted_at,
            updated_at: Utc::now(),
            note: None,
            rating: Rating::Neutral,
            applied_at: None,
            remote: fields.remote.unwrap_or(false),
        }))
    }

    /// db.filter_new gate happened in fetch_source; here db.mark_rejected on
    /// not_job_ad/parse_failed, db.upsert_job, progress line, sleep(pause_ms)
    /// between LLM calls.
    async fn process_candidate(
        &self,
        db: &Db,
        source: &Source,
        candidate: Candidate,
        pause_ms: u64,
        state: &mut FetchState,
        progress: (usize, usize),
    ) -> Result<()> {
        tokio::time::sleep(Duration::from_millis(pause_ms)).await;
        let title = match self.build_job(source, &candidate).await {
            Ok(Some(job)) => {
                match db.upsert_job(&job).await? {
                    UpsertResult::New(_) => state.inc_new(),
                    UpsertResult::Updated(_) | UpsertResult::Duplicate(_) => state.inc_existing(),
                }
                job.title
            }
            Ok(None) => {
                db.mark_rejected(&Platform::Reddit, &candidate.external_id, "not_job_ad")
                    .await?;
                state.inc_skipped();
                candidate.title
            }
            Err(e) => {
                eprintln!("    Warning: failed to process reddit candidate: {e}");
                db.mark_rejected(&Platform::Reddit, &candidate.external_id, "parse_failed")
                    .await?;
                state.inc_skipped();
                candidate.title
            }
        };
        let (idx, total) = progress;
        eprint!("\r    Progress: {idx:>5}/{total:<5} {title:<40.40}");
        Ok(())
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
        self.ensure_reddit_tab(browser).await?;

        let page = browser.new_tab("https://www.reddit.com").await?;
        let mut state = FetchState::new();
        let _guard = CursorGuard::new();

        self.rust_extractor.verify().await?;

        for (i, source) in SOURCES.iter().enumerate() {
            eprintln!(
                "Fetching from {}: {}",
                self.name().underline(),
                format!("https://www.reddit.com/r/{}", source.subreddit).bright_black()
            );
            self.fetch_source(&page, db, source, pause_ms, &mut state)
                .await?;
            // per-source summary; final summary printed by fetch_and_store
            if i + 1 < SOURCES.len() {
                eprintln!("    {}", state.summary());
            }
        }

        page.close().await?;
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_search_fixture_picks_latest_matching() {
        let json: serde_json::Value =
            serde_json::from_str(include_str!("reddit/fixtures/rust_search.json"))
                .expect("fixture should parse");
        let re = Regex::new(r"(?i)who's hiring").expect("regex should compile");
        let id = RedditScraper::pick_megathread(&json, &re);

        assert_eq!(
            id.as_deref(),
            Some("1ttbtf5"),
            "max created_utc matching thread"
        );
    }

    #[test]
    fn test_parse_comments_fixture() {
        let json: serde_json::Value =
            serde_json::from_str(include_str!("reddit/fixtures/rust_comments.json"))
                .expect("fixture should parse");
        let comments = RedditScraper::parse_comments(&json);

        assert_eq!(comments.len(), 2, "top-level t1 only, got: {comments:?}");
        assert!(
            comments.iter().all(|c| c.external_id.starts_with("t1_")),
            "external_id = t1_ name"
        );
        assert!(
            comments.iter().all(|c| c.author != "AutoModerator"),
            "AutoModerator skipped"
        );
        assert!(
            comments.iter().all(|c| c.body != "[deleted]"),
            "[deleted] skipped"
        );
        assert!(
            comments.iter().all(|c| !c.title.is_empty()),
            "title = first non-empty line"
        );
    }

    #[test]
    fn test_title_truncation() {
        let long = "x".repeat(300);
        let truncated = truncate_with_ellipsis(&long, MAX_TITLE_LEN);
        assert_eq!(truncated.chars().count(), MAX_TITLE_LEN + 1);
        assert!(truncated.ends_with('…'));

        let short = "short title";
        assert_eq!(truncate_with_ellipsis(short, MAX_TITLE_LEN), short);
    }
}
