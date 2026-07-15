use crate::db::Db;
use crate::extractors::llm::LlmExtractor;
use crate::extractors::llm_hackernews;
use crate::models::{Data, HackerNewsJobDetail, Job, Platform, Rating};
use crate::platforms::{FetchState, PlatformClient};
use crate::term::CursorGuard;
use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use chromiumoxide::browser::Browser;
use chrono::{DateTime, Utc};
use futures::stream::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::pin::pin;

const ALGOLIA_BASE: &str = "https://hn.algolia.com/api/v1";
const THREAD_QUERY: &str = "Ask HN: Who is hiring";
const DEFAULT_LOCATION: &str = "Europe";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoryHit {
    #[serde(rename = "objectID")]
    object_id: String,
    created_at_i: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StorySearchResponse {
    hits: Vec<StoryHit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentHit {
    #[serde(rename = "objectID")]
    object_id: String,
    created_at_i: i64,
    author: String,
    pub comment_text: String,
    parent_id: i64,
    story_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommentSearchResponse {
    hits: Vec<CommentHit>,
    #[serde(rename = "nbHits")]
    nb_hits: usize,
}

pub struct HackerNewsScraper {
    client: Client,
    extractor: LlmExtractor<llm_hackernews::ExtractFields>,
}

impl HackerNewsScraper {
    #[must_use]
    pub fn new(llm_cli: Option<String>, location: &str) -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (compatible; JobSearch/1.0)")
                .build()
                .unwrap_or_else(|_| Client::new()),
            extractor: LlmExtractor::<llm_hackernews::ExtractFields>::from_cli(llm_cli)
                .with_prompt_context(format!("Candidate location: {location}")),
        }
    }

    async fn latest_thread_id(&self) -> Result<String> {
        let url = format!("{ALGOLIA_BASE}/search_by_date");
        let response: StorySearchResponse = self
            .client
            .get(&url)
            .query(&[
                ("query", THREAD_QUERY),
                ("tags", "story,author_whoishiring"),
                ("hitsPerPage", "1"),
            ])
            .send()
            .await?
            .json()
            .await?;

        response
            .hits
            .into_iter()
            .next()
            .map(|h| h.object_id)
            .ok_or_else(|| anyhow::anyhow!("no Hacker News hiring thread found"))
    }

    async fn fetch_comments_page(
        &self,
        thread_id: i64,
        query: &str,
        page: usize,
    ) -> Result<Vec<CommentHit>> {
        let url = format!("{ALGOLIA_BASE}/search_by_date");
        let response: CommentSearchResponse = self
            .client
            .get(&url)
            .query(&[
                ("query", query),
                ("tags", &format!("comment,story_{thread_id}")),
                ("hitsPerPage", "1000"),
                ("page", &page.to_string()),
            ])
            .send()
            .await?
            .json()
            .await?;
        Ok(response.hits)
    }

    fn html_to_text(html: &str) -> String {
        html2text::from_read(html.as_bytes(), 1000).unwrap_or_else(|_| html.to_string())
    }

    fn title_from_html(html: &str) -> String {
        let text = Self::html_to_text(html);
        text.lines()
            .find(|l| !l.trim().is_empty())
            .map(str::trim)
            .unwrap_or_default()
            .to_string()
    }

    fn truncate_with_ellipsis(text: &str, max_len: usize) -> String {
        if text.chars().count() <= max_len {
            text.to_string()
        } else {
            text.chars().take(max_len).collect::<String>() + "…"
        }
    }

    fn is_flagged(hit: &CommentHit) -> bool {
        hit.comment_text.contains("[flagged]") || hit.comment_text.contains("[dead]")
    }

    async fn build_job(&self, hit: CommentHit) -> Result<Option<Job>> {
        const MAX_TITLE_LEN: usize = 200;

        let body = Self::html_to_text(&hit.comment_text);
        let fields = self.extractor.extract(&body).await?;
        if !fields.is_job_ad {
            return Ok(None);
        }

        let company = fields.company.filter(|s| !s.is_empty());
        let role = fields.role.filter(|s| !s.is_empty());
        let location = fields.location.filter(|s| !s.is_empty());

        let title = role
            .clone()
            .unwrap_or_else(|| Self::title_from_html(&hit.comment_text));
        let title = Self::truncate_with_ellipsis(&title, MAX_TITLE_LEN);

        let remote = fields.remote.unwrap_or(false);
        let tags = fields.tags;
        let budget = fields.budget;

        let posted_at = DateTime::from_timestamp(hit.created_at_i, 0).unwrap_or_else(Utc::now);

        Ok(Some(Job {
            id: 0,
            platform: Platform::Hackernews,
            external_id: hit.object_id.clone(),
            title,
            description: Some(body).filter(|d| !d.is_empty()),
            url: format!("https://news.ycombinator.com/item?id={}", hit.object_id),
            budget,
            tags,
            raw: Data::Hackernews {
                detail: HackerNewsJobDetail {
                    author: hit.author.clone(),
                    author_threads_url: format!(
                        "https://news.ycombinator.com/threads?id={}",
                        urlencoding::encode(&hit.author)
                    ),
                    company,
                    role,
                    location,
                },
            },
            company: None,
            created_at: posted_at,
            updated_at: Utc::now(),
            rating: Rating::Neutral,
            note: None,
            applied_at: None,
            remote,
        }))
    }

    /// Fetch raw top-level comments from the current "Who is hiring?" thread.
    pub async fn fetch_top_level_comments(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<CommentHit>> {
        let thread_id: i64 = self.latest_thread_id().await?.parse()?;
        let max = limit.unwrap_or(usize::MAX);
        let mut top = Vec::new();

        for page in 0usize.. {
            let comments = self.fetch_comments_page(thread_id, query, page).await?;
            let count = comments.len();
            if count == 0 {
                break;
            }
            top.extend(comments.into_iter().filter(|h| h.parent_id == thread_id));
            if top.len() >= max || count < 1000 {
                top.truncate(max);
                break;
            }
        }

        Ok(top)
    }

    fn fetch_new_jobs<'a>(
        &'a self,
        db: &'a Db,
        query: &'a str,
    ) -> impl Stream<Item = Result<Job>> + Send + 'a {
        try_stream! {
            const PENDING_CHUNK: usize = 1000;

            let comments = self.fetch_top_level_comments(query, None).await?;
            eprintln!("    Fetched {} top-level HN comments", comments.len());

            let ids: Vec<String> = comments.iter().map(|h| h.object_id.clone()).collect();
            let mut new_ids = HashSet::new();
            for chunk in ids.chunks(PENDING_CHUNK) {
                new_ids.extend(db.filter_new(&Platform::Hackernews, chunk).await?);
            }

            eprintln!("    {} comments need classification", new_ids.len());

            let new_comments: Vec<_> = comments
                .into_iter()
                .filter(|h| new_ids.contains(&h.object_id))
                .collect();
            if new_comments.is_empty() {
                return;
            }

            self.extractor.verify().await?;

            for hit in new_comments {
                let object_id = hit.object_id.clone();

                if Self::is_flagged(&hit) {
                    db.mark_rejected(&Platform::Hackernews, &object_id, "flagged").await?;
                    continue;
                }

                match self.build_job(hit).await {
                    Ok(Some(job)) => yield job,
                    Ok(None) => {
                        db.mark_rejected(&Platform::Hackernews, &object_id, "not_job_ad")
                            .await?;
                    }
                    Err(e) => {
                        eprintln!("    Warning: failed to parse HN comment: {e}");
                        db.mark_rejected(&Platform::Hackernews, &object_id, "parse_failed")
                            .await?;
                    }
                }
            }
        }
    }

    async fn store_jobs(
        &self,
        db: &Db,
        jobs: impl Stream<Item = Result<Job>>,
    ) -> Result<FetchState> {
        let mut state = FetchState::new();
        let _guard = CursorGuard::new();
        let mut jobs = pin!(jobs);

        while let Some(job) = jobs.next().await {
            let job = job?;
            let should_skip = match &job.raw {
                Data::Hackernews {
                    detail:
                        HackerNewsJobDetail {
                            company: Some(c),
                            role: Some(r),
                            ..
                        },
                } => {
                    let since = chrono::Utc::now() - chrono::Duration::days(60);
                    db.has_similar_hackernews_post(c, r, since).await?
                }
                _ => false,
            };

            if should_skip {
                db.mark_rejected(
                    &Platform::Hackernews,
                    &job.external_id,
                    "similar_recent_job_exists",
                )
                .await?;

                state.inc_existing();
                eprint!("{}", state.progress_line(None, &job.title));
                continue;
            }

            db.upsert_job(&job).await?;
            state.inc_new();
            eprint!("{}", state.progress_line(None, &job.title));
        }

        Ok(state)
    }
}

impl Default for HackerNewsScraper {
    fn default() -> Self {
        Self::new(None, DEFAULT_LOCATION)
    }
}

#[async_trait]
impl PlatformClient for HackerNewsScraper {
    fn name(&self) -> &'static str {
        "hackernews"
    }

    async fn fetch_with_browser(
        &self,
        _browser: &Browser,
        db: &Db,
        _url: &str,
        _pause_ms: u64,
    ) -> Result<FetchState> {
        let jobs = self.fetch_new_jobs(db, "");
        self.store_jobs(db, jobs).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_title_decodes_entities() {
        let html = "SafetyWing (YC W18) | Partnerships Manager | 100% Remote | Hiring Globally&#x27;s team";
        assert_eq!(
            HackerNewsScraper::title_from_html(html),
            "SafetyWing (YC W18) | Partnerships Manager | 100% Remote | Hiring Globally's team"
        );
    }

    #[test]
    fn test_title_strips_tags() {
        let html = "<p>Acme Inc | Rust Engineer | Remote</p><p>Full description here.</p>";
        assert_eq!(
            HackerNewsScraper::title_from_html(html),
            "Acme Inc | Rust Engineer | Remote"
        );
    }
}
