use crate::db::Db;
use crate::extractors::llm::{HackerNewsFields, LlmExtractor};
use crate::models::{Data, HackerNewsJobDetail, Job, Platform};
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
use std::pin::pin;

const ALGOLIA_BASE: &str = "https://hn.algolia.com/api/v1";
const THREAD_QUERY: &str = "Ask HN: Who is hiring";

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
    extractor: LlmExtractor<HackerNewsFields>,
}

impl HackerNewsScraper {
    pub fn new(llm_cli: Option<String>) -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (compatible; JobSearch/1.0)")
                .build()
                .unwrap_or_else(|_| Client::new()),
            extractor: LlmExtractor::<HackerNewsFields>::from_cli(llm_cli),
        }
    }

    async fn latest_thread_id(&self) -> Result<String> {
        let url = format!("{}/search_by_date", ALGOLIA_BASE);
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
        thread_id: &str,
        query: &str,
        page: usize,
    ) -> Result<Vec<CommentHit>> {
        let url = format!("{}/search_by_date", ALGOLIA_BASE);
        let response: CommentSearchResponse = self
            .client
            .get(&url)
            .query(&[
                ("query", query),
                ("tags", &format!("comment,story_{}", thread_id)),
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
        let line = text
            .lines()
            .find(|l| !l.trim().is_empty())
            .map(|l| l.trim())
            .unwrap_or_default();
        Self::truncate_with_ellipsis(line, 200)
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
        if Self::is_flagged(&hit) {
            return Ok(None);
        }

        let body = Self::html_to_text(&hit.comment_text);
        let fields = self.extractor.extract(&body).await?;
        if !fields.is_job_ad {
            return Ok(None);
        }

        let company = fields.company.filter(|s| !s.is_empty());
        let role = fields.role.filter(|s| !s.is_empty());
        let location = fields.location.filter(|s| !s.is_empty());

        const MAX_TITLE_LEN: usize = 200;
        let title = role
            .clone()
            .map(|r| Self::truncate_with_ellipsis(&r, MAX_TITLE_LEN))
            .unwrap_or_else(|| Self::title_from_html(&hit.comment_text));

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
                    author: hit.author,
                    company,
                    role,
                    location,
                    remote,
                },
            },
            company: None,
            created_at: posted_at,
            updated_at: Utc::now(),
            liked: None,
            note: None,
            applied_at: None,
        }))
    }

    /// Fetch raw top-level comments from the current "Who is hiring?" thread.
    pub async fn fetch_top_level_comments(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<CommentHit>> {
        let thread_id = self.latest_thread_id().await?;
        let thread_id_num: i64 = thread_id.parse()?;
        let max = limit.unwrap_or(usize::MAX);
        let mut top = Vec::new();

        for page in 0usize.. {
            let comments = self.fetch_comments_page(&thread_id, query, page).await?;
            let count = comments.len();
            if count == 0 {
                break;
            }
            top.extend(
                comments
                    .into_iter()
                    .filter(|h| h.parent_id == thread_id_num),
            );
            if top.len() >= max || count < 1000 {
                top.truncate(max);
                break;
            }
        }

        Ok(top)
    }

    fn fetch_jobs<'a>(
        &'a self,
        db: &'a Db,
        query: &'a str,
    ) -> impl Stream<Item = Result<Job>> + Send + 'a {
        try_stream! {
            self.extractor.verify().await?;

            let comments = self.fetch_top_level_comments(query, None).await?;

            for hit in comments {
                if db
                    .find_job_id(&Platform::Hackernews, &hit.object_id)
                    .await?
                    .is_some()
                {
                    continue;
                }

                match self.build_job(hit).await {
                    Ok(Some(job)) => yield job,
                    Ok(None) => {}
                    Err(e) => eprintln!("Warning: failed to parse HN comment: {}", e),
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
            let exists = db
                .find_job_id(&Platform::Hackernews, &job.external_id)
                .await?
                .is_some();

            if should_skip || exists {
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
        Self::new(None)
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
        query: &str,
        _pause_ms: u64,
    ) -> Result<FetchState> {
        let jobs = self.fetch_jobs(db, query);
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

    #[test]
    fn test_title_truncates_long_lines() {
        let html = "a".repeat(300);
        let title = HackerNewsScraper::title_from_html(&html);
        assert_eq!(title.chars().count(), 201);
        assert!(title.ends_with('…'));
    }
}
