use crate::browser::{BrowserExt, host_of, wait_for_element, wait_for_with_challenge_recovery};
use crate::db::Db;
use crate::models::{Data, Job, Platform, Rating, UpworkJobDetail};
use crate::platforms::{FetchState, PlatformClient};
use crate::term::CursorGuard;
use anyhow::{Result, anyhow, bail};
use async_trait::async_trait;
use chromiumoxide::browser::Browser;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, sleep};

const FETCH_JOB_DETAIL_JS: &str = include_str!("upwork/fetch_job_detail.js");
const SCRAPE_CARDS_JS: &str = include_str!("upwork/scrape_cards.js");
const HAS_CARDS_JS: &str = include_str!("upwork/has_cards.js");
const HAS_DETAIL_JS: &str = include_str!("upwork/has_detail.js");
const CHALLENGE_JS: &str = include_str!("upwork/challenge.js");
const HAS_NEXT_PAGE_JS: &str = include_str!("upwork/has_next_page.js");

fn format_upwork_budget(s: &str) -> String {
    crate::extractors::budget::parse_upwork_budget(s)
        .map_or_else(|| s.trim().to_string(), |b| b.to_string())
}

fn set_url_page_param(url: &str, page: u32) -> Result<String> {
    let mut parsed = url::Url::parse(url)?;
    let mut pairs: Vec<(String, String)> = parsed
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .filter(|(k, _)| k != "page")
        .collect();
    if page > 1 {
        pairs.push(("page".to_string(), page.to_string()));
    }
    let query = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(pairs)
        .finish();
    if query.is_empty() {
        parsed.set_query(None);
    } else {
        parsed.set_query(Some(&query));
    }
    Ok(parsed.to_string())
}

/// Upwork canonical external id prefix.
const UPWORK_ID_PREFIX: &str = "~02";

/// Normalize Upwork external id to `~02{digits}`.
fn normalize_upwork_external_id(id: &str) -> String {
    let digits = id
        .trim()
        .trim_start_matches(UPWORK_ID_PREFIX)
        .trim_start_matches("02")
        .trim_start_matches('~')
        .trim();
    if digits.is_empty() {
        String::new()
    } else {
        format!("{UPWORK_ID_PREFIX}{digits}")
    }
}

/// Extract external id from either canonical or slugged Upwork job URL.
fn extract_upwork_external_id_from_url(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let segment = parsed.path_segments()?.rfind(|s| !s.is_empty())?;
    segment
        .rsplit('_')
        .find(|part| part.contains('~') || part.chars().any(|c| c.is_ascii_digit()))
        .map(normalize_upwork_external_id)
}

/// Strip slug, query params, and referrer from Upwork job URL.
fn normalize_upwork_url(url: &str) -> String {
    extract_upwork_external_id_from_url(url).map_or_else(
        || url.to_string(),
        |id| format!("https://www.upwork.com/jobs/{id}"),
    )
}

#[derive(Debug, Clone, Deserialize)]
struct RawJobCard {
    pub external_id: String,
    pub title: String,
    pub description: Option<String>,
    pub url: String,
    pub budget: Option<String>,
    #[serde(default)]
    pub posted_at_text: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpworkJobCard {
    pub external_id: String,
    pub title: String,
    pub description: Option<String>,
    pub url: String,
    pub budget: Option<String>,
    pub posted_at_text: Option<DateTime<Utc>>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Clone, Copy)]
struct SearchContext<'a> {
    browser: &'a Browser,
    db: &'a Db,
    pause_ms: u64,
    fetch_started_at: DateTime<Utc>,
    detail_ttl: chrono::Duration,
}

pub struct UpworkScraper;

impl UpworkScraper {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    async fn ensure_upwork_tab(&self, browser: &Browser) -> Result<()> {
        let page_hosts: Vec<_> = browser
            .get_page_urls()
            .await?
            .into_iter()
            .filter_map(|u| host_of(&u))
            .collect();
        if !page_hosts.iter().any(|h| h.contains("upwork.com")) {
            bail!("Upwork requires open upwork.com tab in Brave");
        }
        Ok(())
    }

    pub async fn fetch_job_detail(
        &self,
        browser: &Browser,
        job_url: &str,
    ) -> Result<UpworkJobDetail> {
        let page = browser.new_tab(job_url).await?;

        let detail_loaded = wait_for_with_challenge_recovery(
            &page,
            HAS_DETAIL_JS,
            Some(CHALLENGE_JS),
            None,
            None,
            None,
        )
        .await?;
        if !detail_loaded {
            page.close().await.ok();
            bail!("Job detail page did not load");
        }

        let _ = wait_for_element(&page, &["[data-cy='clock-timelog']"], Some(10), None).await;

        let raw: RawJobDetail = page.evaluate(FETCH_JOB_DETAIL_JS).await?.into_value()?;

        page.close().await.ok();
        raw.try_into()
            .map_err(|e| anyhow!("invalid job detail: {e}"))
    }

    pub async fn scrape_page(page: &chromiumoxide::Page) -> Result<Vec<UpworkJobCard>> {
        let raw: Vec<RawJobCard> = page.evaluate(SCRAPE_CARDS_JS).await?.into_value()?;
        Ok(raw
            .into_iter()
            .map(|r| UpworkJobCard {
                external_id: normalize_upwork_external_id(&r.external_id),
                title: r.title,
                description: r.description,
                url: normalize_upwork_url(&r.url),
                budget: r.budget.map(|b| format_upwork_budget(&b)),
                posted_at_text: crate::models::parse_relative_time(&r.posted_at_text),
                tags: r.tags,
            })
            .collect())
    }

    pub async fn wait_for_jobs(page: &chromiumoxide::Page) -> Result<bool> {
        wait_for_with_challenge_recovery(
            page,
            HAS_CARDS_JS,
            Some(CHALLENGE_JS),
            Some(120),
            Some(Duration::from_millis(500)),
            None,
        )
        .await
    }

    async fn process_search_card(
        &self,
        ctx: SearchContext<'_>,
        v: &UpworkJobCard,
        state: &mut FetchState,
    ) -> Result<()> {
        let is_stale = v.posted_at_text.is_some_and(|posted| {
            let age = chrono::Utc::now() - posted;
            age.num_days() >= 7
        });

        let updated_at = ctx
            .db
            .job_updated_at(&Platform::Upwork, &v.external_id)
            .await?;

        if let Some(ts) = updated_at
            && (ctx.fetch_started_at - ts < ctx.detail_ttl || is_stale)
        {
            state.inc_existing();
            eprint!("{}", state.progress_line(None, ""));
            return Ok(());
        }

        let job_url = v.url.clone();

        match self.fetch_job_detail(ctx.browser, &job_url).await {
            Ok(detail) => {
                let job = Job {
                    id: 0,
                    platform: Platform::Upwork,
                    external_id: v.external_id.clone(),
                    title: v.title.clone(),
                    description: v.description.clone(),
                    url: v.url.clone(),
                    budget: v.budget.clone(),
                    tags: v.tags.clone(),
                    raw: Data::Upwork { detail },
                    company: None,
                    created_at: v.posted_at_text.unwrap_or_else(chrono::Utc::now),
                    updated_at: chrono::Utc::now(),
                    rating: Rating::Neutral,
                    note: None,
                    applied_at: None,
                    remote: true,
                };
                ctx.db.upsert_job(&job).await?;

                if updated_at.is_some() {
                    state.inc_existing();
                } else {
                    state.inc_new();
                }
            }
            Err(e) => {
                eprintln!("    Warning: failed to fetch detail for {}: {}", v.title, e);
            }
        }

        eprint!("{}", state.progress_line(None, &v.title));
        sleep(Duration::from_millis(ctx.pause_ms)).await;
        Ok(())
    }
}

impl Default for UpworkScraper {
    fn default() -> Self {
        Self
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawJobDetail {
    pub proposals: String,
    pub last_viewed: String,
    pub interviewing: String,
    pub invites_sent: String,
    pub unanswered_invites: String,
    pub description: String,
    pub exact_budget: String,
    pub experience_level: String,
    pub hires: String,
    pub project_type: String,
    pub duration: String,
    pub hours_per_week: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub posted_at_text: String,
}

impl TryFrom<RawJobDetail> for UpworkJobDetail {
    type Error = anyhow::Error;

    fn try_from(raw: RawJobDetail) -> Result<Self, Self::Error> {
        Ok(UpworkJobDetail {
            proposals: raw.proposals,
            last_viewed: crate::models::parse_relative_time(&raw.last_viewed),
            interviewing: raw.interviewing,
            invites_sent: raw.invites_sent,
            unanswered_invites: raw.unanswered_invites,
            description: raw.description,
            exact_budget: format_upwork_budget(&raw.exact_budget),
            experience_level: raw.experience_level,
            hires: raw.hires,
            project_type: raw.project_type,
            duration: raw.duration,
            hours_per_week: raw.hours_per_week,
            tags: raw.tags,
            posted_at: crate::models::parse_relative_time(&raw.posted_at_text)
                .unwrap_or_else(Utc::now),
        })
    }
}

#[async_trait]
impl PlatformClient for UpworkScraper {
    fn name(&self) -> &'static str {
        "upwork"
    }

    async fn fetch_with_browser(
        &self,
        browser: &Browser,
        db: &Db,
        url: &str,
        pause_ms: u64,
    ) -> Result<FetchState> {
        self.ensure_upwork_tab(browser).await?;

        let parsed =
            url::Url::parse(url).map_err(|e| anyhow::anyhow!("invalid Upwork URL: {e}"))?;
        let host = parsed.host_str().unwrap_or_default();
        if !host.ends_with("upwork.com") {
            bail!("Upwork URL must be on upwork.com subdomain");
        }

        let page = browser.new_tab(url).await?;

        if !Self::wait_for_jobs(&page).await? {
            bail!("Upwork job cards did not appear. Login at upwork.com in Brave first.");
        }

        sleep(Duration::from_millis(pause_ms)).await;

        let mut page_num = 1u32;
        let mut state = FetchState::new();
        let fetch_started_at = chrono::Utc::now();
        let detail_ttl = chrono::Duration::hours(24);

        let _guard = CursorGuard::new();

        loop {
            let raw_jobs = Self::scrape_page(&page).await?;

            let ctx = SearchContext {
                browser,
                db,
                pause_ms,
                fetch_started_at,
                detail_ttl,
            };
            for v in &raw_jobs {
                self.process_search_card(ctx, v, &mut state).await?;
            }

            let has_next: bool = page.evaluate(HAS_NEXT_PAGE_JS).await?.into_value()?;

            if !has_next {
                break;
            }

            page_num += 1;
            let next_url = set_url_page_param(url, page_num)
                .map_err(|e| anyhow::anyhow!("failed to build next page URL: {e}"))?;
            page.goto(&next_url).await?;
            page.wait_for_navigation().await?;

            if !Self::wait_for_jobs(&page).await? {
                break;
            }
            sleep(Duration::from_millis(pause_ms)).await;
        }
        page.close().await.ok();
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_upwork_external_id_variants() {
        assert_eq!(
            normalize_upwork_external_id("2062803789757972368"),
            "~022062803789757972368"
        );
        assert_eq!(
            normalize_upwork_external_id("~022062803789757972368"),
            "~022062803789757972368"
        );
        assert_eq!(
            normalize_upwork_external_id("022062803789757972368"),
            "~022062803789757972368"
        );
        assert_eq!(
            normalize_upwork_external_id("  ~022062803789757972368  "),
            "~022062803789757972368"
        );
        assert_eq!(normalize_upwork_external_id(""), "");
    }

    #[test]
    fn test_extract_upwork_external_id_from_slug_url() {
        let url = "https://www.upwork.com/jobs/Kalshi-span-class-highlight-Trading-span-Bot-Developer-Python-Prediction-Markets_~022062803789757972368/?referrer_url_path=/nx/search/jobs/";
        assert_eq!(
            extract_upwork_external_id_from_url(url),
            Some("~022062803789757972368".to_string())
        );
    }

    #[test]
    fn test_normalize_upwork_url_strips_slug_and_query() {
        let slug = "https://www.upwork.com/jobs/Kalshi-span-class-highlight-Trading-span-Bot-Developer-Python-Prediction-Markets_~022062803789757972368/?referrer_url_path=/nx/search/jobs/";
        assert_eq!(
            normalize_upwork_url(slug),
            "https://www.upwork.com/jobs/~022062803789757972368"
        );
        assert_eq!(
            normalize_upwork_url("https://www.upwork.com/jobs/~022062803789757972368"),
            "https://www.upwork.com/jobs/~022062803789757972368"
        );
    }

    #[test]
    fn test_set_url_page_param() {
        let base = "https://www.upwork.com/nx/search/jobs/?q=rust&sort=recency&per_page=50&t=0";
        let url = set_url_page_param(base, 2).unwrap();
        assert!(url.contains("page=2"));
        assert!(url.contains("q=rust"));
        let url = set_url_page_param(base, 1).unwrap();
        assert!(!url.contains("page=1"));
        assert!(url.contains("q=rust"));
    }
}
