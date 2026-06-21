use crate::browser::BrowserManager;
use crate::db::Db;
use crate::extractors::llm::{HackerNewsFields, LlmExtractor};
use crate::models::{Data, HackerNewsJobDetail, Job, Platform};
use crate::platforms::{FetchState, PlatformClient};
use crate::term::CursorGuard;
use anyhow::Result;
use async_trait::async_trait;
use chromiumoxide::browser::Browser;
use chrono::{DateTime, Utc};
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

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
struct CommentHit {
    #[serde(rename = "objectID")]
    object_id: String,
    created_at_i: i64,
    author: String,
    comment_text: String,
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

static JOB_KEYWORDS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(engineer|engineers|developer|developers|manager|managers|designer|designers|scientist|scientists|analyst|analysts|founder|founders|lead|leads|architect|architects|researcher|researchers|writer|writers|coordinator|coordinators|specialist|specialists|intern|interns|head of|vp of|director|directors|fullstack|full-stack|full stack|frontend|backend|devops|sre|ml|ai|data|product|security|support|sales|marketing|legal|operations)\b",
    )
    .unwrap()
});

static LOCATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(remote|onsite|hybrid|distributed|worldwide|global)\b").unwrap()
});

static SALARY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)[\$€£]|\b(usd|eur|gbp|cad|aud)\b|\d+\s*[kKmM]\b|\bequity\b").unwrap()
});

static URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)https?://|apply\s*[:\-]?|jobs\.|careers\.|lever\.co|ashbyhq\.com|greenhouse\.io|workday|breezy\.hr|tally\.so|forms\.gle|wellfound\.com").unwrap()
});

impl HackerNewsScraper {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (compatible; JobSearch/1.0)")
                .build()
                .unwrap_or_else(|_| Client::new()),
            extractor: LlmExtractor::<HackerNewsFields>::from_env(),
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

    async fn fetch_comments(&self, thread_id: &str, query: &str) -> Result<Vec<CommentHit>> {
        let mut all = Vec::new();
        let mut page = 0;

        loop {
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

            let got = response.hits.len();
            if got == 0 {
                break;
            }
            all.extend(response.hits);
            if got < 1000 {
                break;
            }
            page += 1;
        }

        Ok(all)
    }

    fn html_to_text(html: &str) -> String {
        html2text::from_read(html.as_bytes(), 1000).unwrap_or_else(|_| html.to_string())
    }

    fn normalize_text(html: &str) -> String {
        let text = Self::html_to_text(html);
        regex::Regex::new(r"\s+")
            .unwrap()
            .replace_all(&text, " ")
            .trim()
            .to_string()
    }

    fn first_line(html: &str) -> String {
        Self::html_to_text(html)
            .lines()
            .find(|l| !l.trim().is_empty())
            .map(|l| l.trim().to_string())
            .unwrap_or_default()
    }

    const MAX_TITLE_LEN: usize = 200;

    fn truncate_title(s: &str) -> String {
        if s.chars().count() <= Self::MAX_TITLE_LEN {
            s.to_string()
        } else {
            s.chars().take(Self::MAX_TITLE_LEN).collect::<String>() + "…"
        }
    }

    #[cfg(test)]
    fn is_remote(text: &str) -> bool {
        let lower = text.to_lowercase();
        lower.contains("remote")
            || lower.contains("distributed")
            || lower.contains("worldwide")
            || lower.contains("global")
    }

    fn job_score(first: &str, full: &str) -> i32 {
        let lower_first = first.to_lowercase();
        let mut score = 0;

        if first.contains('|') {
            score += 2;
        }
        if JOB_KEYWORDS_RE.is_match(&lower_first) {
            score += 2;
        }
        if LOCATION_RE.is_match(&lower_first) {
            score += 1;
        }
        if SALARY_RE.is_match(&lower_first) || SALARY_RE.is_match(full) {
            score += 1;
        }
        if URL_RE.is_match(&lower_first) || URL_RE.is_match(full) {
            score += 1;
        }
        if first.split_whitespace().count() <= 40 {
            score += 1;
        }

        score
    }

    fn is_job_post(hit: &CommentHit) -> bool {
        let text = hit.comment_text.trim();
        if text.contains("[flagged]") || text.contains("[dead]") {
            return false;
        }

        let first = Self::first_line(&hit.comment_text);
        let full = Self::normalize_text(&hit.comment_text);
        Self::job_score(&first, &full) >= 3
    }

    fn split_first_line(first: &str) -> (Option<String>, Option<String>, Option<String>) {
        let parts: Vec<&str> = first.split('|').map(str::trim).collect();
        if parts.len() >= 2 {
            let company = if parts[0].is_empty() {
                None
            } else {
                Some(parts[0].to_string())
            };
            let role = if parts[1].is_empty() {
                None
            } else {
                Some(parts[1].to_string())
            };
            let location = parts.get(2).and_then(|s| {
                let s = s.trim();
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            });
            (company, role, location)
        } else {
            (None, None, None)
        }
    }

    async fn build_job(&self, hit: CommentHit) -> Result<Option<Job>> {
        let first = Self::first_line(&hit.comment_text);
        let body = Self::html_to_text(&hit.comment_text);

        let fields = self.extractor.extract(&body).await?;
        if !fields.is_job_ad {
            return Ok(None);
        }

        let (fallback_company, fallback_role, fallback_location) = Self::split_first_line(&first);
        let company = fields
            .company
            .filter(|s| !s.is_empty())
            .or(fallback_company);
        let role = fields.role.filter(|s| !s.is_empty()).or(fallback_role);
        let location = fields
            .location
            .filter(|s| !s.is_empty())
            .or(fallback_location);

        let remote = fields.remote.unwrap_or(false);
        let tags = fields.tags;
        let budget = fields.budget;

        let posted_at = DateTime::from_timestamp(hit.created_at_i, 0).unwrap_or_else(Utc::now);

        Ok(Some(Job {
            id: 0,
            platform: Platform::Hackernews,
            external_id: hit.object_id.clone(),
            title: Self::truncate_title(&first),
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

    pub async fn fetch_jobs(&self, query: &str) -> Result<Vec<Job>> {
        let thread_id = self.latest_thread_id().await?;
        let comments = self.fetch_comments(&thread_id, query).await?;

        let thread_id_num: i64 = thread_id.parse()?;
        let mut jobs = Vec::new();

        for hit in comments {
            if hit.parent_id != thread_id_num {
                continue;
            }
            if !Self::is_job_post(&hit) {
                continue;
            }
            match self.build_job(hit).await {
                Ok(Some(job)) => jobs.push(job),
                Ok(None) => {}
                Err(e) => eprintln!("Warning: failed to parse HN comment: {}", e),
            }
        }

        Ok(jobs)
    }

    async fn run_fetch(&self, db: &Db, query: &str) -> Result<FetchState> {
        let jobs = self.fetch_jobs(query).await?;
        let mut state = FetchState::new();
        let total = jobs.len();
        let _guard = CursorGuard::new();

        for job in jobs {
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
                eprint!("{}", state.progress_line(Some(total), &job.title));
                continue;
            }

            db.upsert_job(&job).await?;
            state.inc_new();
            eprint!("{}", state.progress_line(Some(total), &job.title));
        }

        Ok(state)
    }
}

impl Default for HackerNewsScraper {
    fn default() -> Self {
        Self::new()
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
        self.run_fetch(db, query).await
    }

    async fn fetch_with_manager(
        &self,
        _manager: &BrowserManager,
        db: &Db,
        query: &str,
        _pause_ms: u64,
    ) -> Result<FetchState> {
        self.run_fetch(db, query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_line_decodes_entities() {
        let html = "SafetyWing (YC W18) | Partnerships Manager | 100% Remote | Hiring Globally&#x27;s team";
        assert_eq!(
            HackerNewsScraper::first_line(html),
            "SafetyWing (YC W18) | Partnerships Manager | 100% Remote | Hiring Globally's team"
        );
    }

    #[test]
    fn test_first_line_strips_tags() {
        let html = "<p>Acme Inc | Rust Engineer | Remote</p><p>Full description here.</p>";
        assert_eq!(
            HackerNewsScraper::first_line(html),
            "Acme Inc | Rust Engineer | Remote"
        );
    }

    #[test]
    fn test_job_score_real_post() {
        let first = "Upwave (YC S12) | Senior Software Engineer | REMOTE (US) | Full-time | $150k-$175k + bonus";
        let full = first;
        assert!(HackerNewsScraper::job_score(first, full) >= 3);
    }

    #[test]
    fn test_job_score_meta_comment_rejected() {
        let first = "Noticing a pattern across this month's thread: lots of posts this time";
        let full = first;
        assert!(HackerNewsScraper::job_score(first, full) < 3);
    }

    #[test]
    fn test_job_score_flagged_rejected() {
        let hit = CommentHit {
            object_id: "123".to_string(),
            created_at_i: 0,
            author: "x".to_string(),
            comment_text: "[flagged]".to_string(),
            parent_id: 1,
            story_id: 1,
        };
        assert!(!HackerNewsScraper::is_job_post(&hit));
    }

    #[test]
    fn test_is_remote_detects_variants() {
        assert!(HackerNewsScraper::is_remote(
            "Remote OK, distributed team, worldwide"
        ));
        assert!(!HackerNewsScraper::is_remote("Onsite in NYC"));
    }
}
