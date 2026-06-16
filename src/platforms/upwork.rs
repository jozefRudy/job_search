use crate::browser::{BrowserExt, host_of, wait_for_element};
use crate::db::Db;
use crate::models::{Budget, Data, Job, Platform, UpworkJobDetail};
use crate::platforms::{FetchState, PlatformClient};
use crate::term::CursorGuard;
use anyhow::{Result, anyhow, bail};
use async_trait::async_trait;
use chromiumoxide::browser::Browser;
use chrono::{DateTime, Utc};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::cmp::min;
use tokio::time::{Duration, sleep};

const FETCH_JOB_DETAIL_JS: &str = include_str!("upwork/fetch_job_detail.js");
const SCRAPE_CARDS_JS: &str = include_str!("upwork/scrape_cards.js");
const IS_CHALLENGE_JS: &str = include_str!("upwork/is_challenge.js");
const HAS_CARDS_JS: &str = include_str!("upwork/has_cards.js");
const HAS_NEXT_PAGE_JS: &str = include_str!("upwork/has_next_page.js");
const EXTRACT_SUBMITTED_LIST_JS: &str = include_str!("upwork/extract_submitted_list.js");
const EXTRACT_PROPOSAL_DETAIL_JS: &str = include_str!("upwork/extract_proposal_detail.js");
const CLICK_PAGE_JS: &str = include_str!("upwork/click_page.js");
const GET_SUBMITTED_PAGE_JS: &str = include_str!("upwork/get_submitted_page.js");

fn parse_upwork_budget(s: &str) -> String {
    let lower = s.to_lowercase();
    let period = if lower.contains("hour") {
        Some("hr")
    } else {
        None
    };
    let cleaned = s
        .replace("Hourly:", "")
        .replace("hourly:", "")
        .replace("Fixed-price:", "")
        .replace("fixed-price:", "")
        .replace("Fixed price:", "")
        .replace("fixed price:", "");
    Budget::parse(cleaned.trim(), period)
        .map(|b| b.to_string())
        .unwrap_or_else(|| s.trim().to_string())
}

/// Upwork canonical external id prefix.
const UPWORK_ID_PREFIX: &str = "~02";

/// Normalize Upwork external id to `~02{digits}`.
/// Handles ids from search (`data-ev-job-uid`) and from proposals API (`openingUID`).
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
        format!("{}{}", UPWORK_ID_PREFIX, digits)
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
    extract_upwork_external_id_from_url(url)
        .map(|id| format!("https://www.upwork.com/jobs/{}", id))
        .unwrap_or_else(|| url.to_string())
}

/// Raw job card from JS scraper (all strings).
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

/// Job card with parsed timestamps.
#[derive(Debug, Clone, Deserialize)]
#[allow(non_snake_case)]
struct RawSubmittedList {
    pub page: u32,
    pub total: u32,
    pub itemsPerPage: u32,
    pub items: Vec<RawSubmittedItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(non_snake_case)]
struct RawSubmittedItem {
    pub openingUID: String,
    pub applicationUID: String,
    pub title: String,
    pub createdTs: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Default)]
pub struct UpworkSearchParams {
    pub query: String,
    pub tier: Option<UpworkTier>,
    pub hourly_rate_min: Option<u32>,
    pub client_hires: Option<String>,
    pub page: u32,
}

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
#[clap(rename_all = "kebab")]
pub enum UpworkTier {
    #[default]
    All,
    Expert,
    Intermediate,
    BothUpper,
}

impl UpworkSearchParams {
    pub fn new(query: &str) -> Self {
        Self {
            query: query.to_string(),
            page: 1,
            ..Default::default()
        }
    }

    pub fn tier(mut self, tier: Option<UpworkTier>) -> Self {
        self.tier = tier;
        self
    }

    pub fn hourly_rate_min(mut self, min: Option<u32>) -> Self {
        self.hourly_rate_min = min;
        self
    }

    pub fn client_hires(mut self, hires: Option<String>) -> Self {
        self.client_hires = hires;
        self
    }

    pub fn page(mut self, page: u32) -> Self {
        self.page = page;
        self
    }

    pub fn build_url(&self) -> String {
        let mut url = url::Url::parse("https://www.upwork.com/nx/search/jobs/").unwrap();
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("q", &self.query);
            pairs.append_pair("sort", "recency");
            pairs.append_pair("per_page", "50");
            if self.page > 1 {
                pairs.append_pair("page", &self.page.to_string());
            }
            if let Some(tier) = self.tier {
                match tier {
                    UpworkTier::Expert => {
                        pairs.append_pair("contractor_tier", "3");
                    }
                    UpworkTier::Intermediate => {
                        pairs.append_pair("contractor_tier", "2");
                    }
                    UpworkTier::BothUpper => {
                        pairs.append_pair("contractor_tier", "2,3");
                    }
                    UpworkTier::All => {}
                }
            }
            pairs.append_pair("t", "0");
            if let Some(min) = self.hourly_rate_min {
                pairs.append_pair("hourly_rate", &format!("{}-", min));
            }
            if let Some(ref hires) = self.client_hires {
                pairs.append_pair("client_hires", hires);
            }
        }
        url.to_string()
    }
}

#[derive(Debug, Clone, Default)]
pub struct UpworkScraper {
    pub tier: Option<UpworkTier>,
    pub hourly_rate_min: Option<u32>,
    pub client_hires: Option<String>,
}

impl UpworkScraper {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(
        tier: Option<UpworkTier>,
        hourly_rate_min: Option<u32>,
        client_hires: Option<String>,
    ) -> Self {
        Self {
            tier,
            hourly_rate_min,
            client_hires,
        }
    }

    pub fn build_search_url(
        query: &str,
        tier: Option<UpworkTier>,
        hourly_rate_min: Option<u32>,
        client_hires: Option<String>,
        page: u32,
    ) -> String {
        UpworkSearchParams::new(query)
            .tier(tier)
            .hourly_rate_min(hourly_rate_min)
            .client_hires(client_hires)
            .page(page)
            .build_url()
    }

    pub async fn fetch_job_detail(
        &self,
        browser: &Browser,
        job_url: &str,
    ) -> Result<UpworkJobDetail> {
        let page = browser.new_tab(job_url).await?;

        if !wait_for_element(
            &page,
            &["[data-test='Description']", "[class*='description']"],
            None,
            None,
        )
        .await?
        {
            page.close().await.ok();
            bail!("Job detail page did not load");
        }

        // Best-effort wait for budget panel to render (fixed jobs may never show it).
        let _ = wait_for_element(&page, &["[data-cy='clock-timelog']"], Some(10), None).await;

        let raw: RawJobDetail = page.evaluate(FETCH_JOB_DETAIL_JS).await?.into_value()?;

        page.close().await.ok();
        raw.try_into()
            .map_err(|e| anyhow!("invalid job detail: {}", e))
    }

    /// Scrape job cards from current search page.
    pub async fn scrape_page(page: &chromiumoxide::Page) -> Result<Vec<UpworkJobCard>> {
        let raw: Vec<RawJobCard> = page.evaluate(SCRAPE_CARDS_JS).await?.into_value()?;
        Ok(raw
            .into_iter()
            .map(|r| UpworkJobCard {
                external_id: normalize_upwork_external_id(&r.external_id),
                title: r.title,
                description: r.description,
                url: normalize_upwork_url(&r.url),
                budget: r.budget.map(|b| parse_upwork_budget(&b)),
                posted_at_text: crate::models::parse_relative_time(&r.posted_at_text),
                tags: r.tags,
            })
            .collect())
    }

    /// Wait for job cards to appear (or CAPTCHA). Returns true if cards found.
    pub async fn wait_for_jobs(page: &chromiumoxide::Page) -> Result<bool> {
        for i in 0..120 {
            let is_challenge: bool = page.evaluate(IS_CHALLENGE_JS).await?.into_value()?;

            let has_cards: bool = page.evaluate(HAS_CARDS_JS).await?.into_value()?;

            if !is_challenge && has_cards {
                return Ok(true);
            }

            if i == 30 {
                eprintln!("  Upwork showing CAPTCHA. Login in Brave first, then retry.");
            }
            sleep(Duration::from_millis(500)).await;
        }
        Ok(false)
    }
}

/// Raw detail from JS scraper (all strings).
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
            exact_budget: parse_upwork_budget(&raw.exact_budget),
            experience_level: raw.experience_level,
            hires: raw.hires,
            project_type: raw.project_type,
            duration: raw.duration,
            hours_per_week: raw.hours_per_week,
            tags: raw.tags,
            posted_at: crate::models::parse_relative_time(&raw.posted_at_text),
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
        query: &str,
        pause_ms: u64,
    ) -> Result<FetchState> {
        let page_hosts: Vec<_> = browser
            .get_page_urls()
            .await?
            .into_iter()
            .filter_map(|u| host_of(&u))
            .collect();
        if !page_hosts.iter().any(|h| h.contains("upwork.com")) {
            bail!("Upwork requires open upwork.com tab in Brave");
        }

        let base_params = UpworkSearchParams::new(query)
            .tier(self.tier)
            .hourly_rate_min(self.hourly_rate_min)
            .client_hires(self.client_hires.clone());

        let search_url = base_params.clone().build_url();
        let page = browser.new_tab(&search_url).await?;

        if !Self::wait_for_jobs(&page).await? {
            bail!("Upwork job cards did not appear. Login at upwork.com in Brave first.");
        }

        sleep(Duration::from_millis(pause_ms)).await;

        let mut all_jobs: Vec<Job> = Vec::new();
        let mut page_num = 1u32;
        let mut state = FetchState::new();
        let fetch_started_at = chrono::Utc::now();
        let detail_ttl = chrono::Duration::hours(24);

        let _guard = CursorGuard::new();

        loop {
            let raw_jobs = Self::scrape_page(&page).await?;

            for v in &raw_jobs {
                let is_stale = v.posted_at_text.is_some_and(|posted| {
                    let age = chrono::Utc::now() - posted;
                    age.num_days() >= 7
                });

                let updated_at = db.job_updated_at(&Platform::Upwork, &v.external_id).await?;

                if let Some(ts) = updated_at
                    && (fetch_started_at - ts < detail_ttl || is_stale)
                {
                    state.inc_existing();
                    eprint!("{}", state.progress_line(None, ""));
                    continue;
                }

                let job_url = v.url.clone();

                match self.fetch_job_detail(browser, &job_url).await {
                    Ok(detail) => {
                        let job = Job {
                            id: None,
                            platform: Platform::Upwork,
                            external_id: v.external_id.clone(),
                            title: v.title.clone(),
                            description: v.description.clone(),
                            url: v.url.clone(),
                            budget: v.budget.clone(),
                            tags: v.tags.clone(),
                            raw: Data::Upwork { detail },
                            created_at: v.posted_at_text.unwrap_or_else(chrono::Utc::now),
                            updated_at: chrono::Utc::now(),
                            liked: None,
                            note: None,
                            applied_at: None,
                        };
                        db.upsert_job(&job).await?;

                        let exists = updated_at.is_some();
                        if exists {
                            state.inc_existing();
                        } else {
                            state.inc_new();
                        }
                        all_jobs.push(job);
                    }
                    Err(e) => {
                        eprintln!("    Warning: failed to fetch detail for {}: {}", v.title, e);
                    }
                }

                eprint!("{}", state.progress_line(None, &v.title));
                sleep(Duration::from_millis(pause_ms)).await;
            }

            let has_next: bool = page.evaluate(HAS_NEXT_PAGE_JS).await?.into_value()?;

            if !has_next {
                break;
            }

            page_num += 1;
            let next_url = base_params.clone().page(page_num).build_url();
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

    async fn sync_applications(
        &self,
        browser: &Browser,
        db: &Db,
        pause_ms: u64,
        limit: Option<usize>,
    ) -> Result<FetchState> {
        let page_hosts: Vec<_> = browser
            .get_page_urls()
            .await?
            .into_iter()
            .filter_map(|u| host_of(&u))
            .collect();
        if !page_hosts.iter().any(|h| h.contains("upwork.com")) {
            bail!("Upwork requires open upwork.com tab in Brave");
        }

        let page = browser
            .new_tab("https://www.upwork.com/nx/proposals/")
            .await?;
        sleep(Duration::from_millis(pause_ms)).await;

        let mut all_proposals: Vec<RawSubmittedItem> = Vec::new();
        let max = limit.unwrap_or(usize::MAX);

        let _guard = CursorGuard::new();
        loop {
            let list: RawSubmittedList = page
                .evaluate(EXTRACT_SUBMITTED_LIST_JS)
                .await?
                .into_value()?;
            all_proposals.extend(list.items);

            let total_pages = list.total.div_ceil(list.itemsPerPage);
            let next_page = list.page + 2; // page is 0-indexed, button is 1-indexed

            if next_page > total_pages || all_proposals.len() >= max {
                break;
            }

            let click_js = CLICK_PAGE_JS.replace("{page}", &next_page.to_string());
            page.evaluate(click_js.as_str()).await?;
            sleep(Duration::from_millis(pause_ms)).await;

            // Verify page changed
            let new_page: u32 = page.evaluate(GET_SUBMITTED_PAGE_JS).await?.into_value()?;
            if new_page <= list.page {
                break;
            }
        }

        page.close().await.ok();

        let mut state = FetchState::new();
        let total = limit
            .map(|l| min(l, all_proposals.len()))
            .unwrap_or(all_proposals.len());

        for item in &all_proposals {
            if state.checked() >= max {
                break;
            }
            let external_id = normalize_upwork_external_id(&item.openingUID);
            let job_url = format!("https://www.upwork.com/jobs/{}", external_id);

            let job_id = if let Some(id) = db.find_job_id(&Platform::Upwork, &external_id).await? {
                Some(id)
            } else {
                match self.fetch_job_detail(browser, &job_url).await {
                    Ok(detail) => {
                        let created_at = detail.posted_at.unwrap_or_else(chrono::Utc::now);
                        let job = Job {
                            id: None,
                            platform: Platform::Upwork,
                            external_id: external_id.clone(),
                            title: item.title.clone(),
                            description: Some(detail.description.clone()).filter(|b| !b.is_empty()),
                            url: job_url.clone(),
                            budget: Some(parse_upwork_budget(&detail.exact_budget))
                                .filter(|b| !b.is_empty()),
                            tags: detail.tags.clone(),
                            raw: Data::Upwork { detail },
                            created_at,
                            updated_at: chrono::Utc::now(),
                            liked: None,
                            note: None,
                            applied_at: None,
                        };
                        Some(db.upsert_job(&job).await?)
                    }
                    Err(e) => {
                        eprintln!(
                            "  Warning: failed to fetch detail for {}: {}",
                            item.title, e
                        );
                        None
                    }
                }
            };

            let Some(job_id) = job_id else { continue };

            if db
                .get_job(job_id)
                .await?
                .and_then(|j| j.applied_at)
                .is_some()
            {
                state.inc_existing();
                eprint!("{}", state.progress_line(Some(total), &item.title));
                continue;
            }

            let cover_letter: String = {
                let detail_page = browser
                    .new_tab(&format!(
                        "https://www.upwork.com/nx/proposals/{}",
                        item.applicationUID
                    ))
                    .await?;
                sleep(Duration::from_millis(pause_ms)).await;
                let letter = detail_page
                    .evaluate(EXTRACT_PROPOSAL_DETAIL_JS)
                    .await?
                    .into_value::<String>()?;
                detail_page.close().await.ok();
                letter
            };

            let applied_at = item
                .createdTs
                .as_ref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            let note = if cover_letter.is_empty() {
                None
            } else {
                Some(cover_letter.as_str())
            };

            db.set_applied(job_id, note, applied_at).await?;
            state.inc_new();

            eprint!("{}", state.progress_line(Some(total), &item.title));
        }
        Ok(state)
    }

    async fn react(&self, _job: &Job, _note: Option<String>) -> Result<()> {
        Err(anyhow!("Upwork react not yet implemented"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_search_url() {
        let url = UpworkScraper::build_search_url("quant trading", None, None, None, 1);
        assert!(url.contains("/nx/search/jobs/"));
        assert!(url.contains("q=quant+trading"));
        assert!(url.contains("sort=recency"));
        assert!(url.contains("per_page=50"));
        assert!(url.contains("t=0"));
    }

    #[test]
    fn test_build_search_url_with_filters() {
        let url = UpworkScraper::build_search_url(
            "quant trading",
            Some(UpworkTier::BothUpper),
            Some(65),
            Some("1-9,10-".to_string()),
            2,
        );
        assert!(url.contains("/nx/search/jobs/"));
        assert!(url.contains("q=quant+trading"));
        assert!(url.contains("contractor_tier=2%2C3"));
        assert!(url.contains("hourly_rate=65-"));
        assert!(url.contains("t=0"));
        assert!(url.contains("client_hires=1-9%2C10-"));
        assert!(url.contains("page=2"));
    }

    #[test]
    fn test_upwork_search_params_defaults() {
        let params = UpworkSearchParams::new("rust");
        let url = params.build_url();
        assert_eq!(
            url,
            "https://www.upwork.com/nx/search/jobs/?q=rust&sort=recency&per_page=50&t=0"
        );
    }

    #[test]
    fn test_upwork_search_params_builder_applies_filters() {
        let url = UpworkSearchParams::new("rust")
            .tier(Some(UpworkTier::Expert))
            .hourly_rate_min(Some(100))
            .client_hires(Some("10-".to_string()))
            .page(3)
            .build_url();
        assert!(url.contains("contractor_tier=3"));
        assert!(url.contains("hourly_rate=100-"));
        assert!(url.contains("t=0"));
        assert!(url.contains("client_hires=10-"));
        assert!(url.contains("page=3"));
    }

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
}
