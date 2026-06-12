use crate::browser::BrowserExt;
use crate::db::Db;
use crate::models::{Budget, Data, Job, NoFluffJobDetail, Platform};
use crate::platforms::PlatformClient;
use crate::term::CursorGuard;
use anyhow::{Result, bail};
use async_trait::async_trait;
use chromiumoxide::browser::Browser;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::min;
use std::collections::{HashMap, HashSet};
use tokio::time::{Duration, sleep};

const SCRAPE_CARDS_JS: &str = include_str!("nofluffjobs/scrape_cards.js");
const CLICK_LOAD_MORE_JS: &str = include_str!("nofluffjobs/click_load_more.js");
const COUNT_CARDS_JS: &str = include_str!("nofluffjobs/count_cards.js");
const GET_TOTAL_RESULTS_JS: &str = include_str!("nofluffjobs/get_total_results.js");
const FETCH_APPLICATIONS_JS: &str = include_str!("nofluffjobs/fetch_applications.js");

/// Card scraped from NoFluffJobs search page DOM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NofluffJobCard {
    pub external_id: String,
    pub title: String,
    pub url: String,
    pub budget: Option<String>,
    pub tags: Vec<String>,
}

pub struct NoFluffJobsScraper {
    config: NoFluffJobsConfig,
    client: Client,
}

const API_BASE: &str = "https://nofluffjobs.com/api";

/// Raw API response from NoFluffJobs /candidates/my-applications endpoint.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawApplicationsResponse {
    has_next: bool,
    items: Vec<RawApplicationItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawApplicationItem {
    posting_id: String,
    applied_date: NfjDate,
    #[serde(default)]
    status_history: Vec<RawStatusHistoryEntry>,
    offer: RawOfferSummary,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawStatusHistoryEntry {
    status: String,
    date: NfjDate,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawOfferSummary {
    title: String,
    #[serde(default)]
    salary: Option<RawSalary>,
    #[serde(default)]
    tiles: RawTiles,
    url: String,
    #[serde(default)]
    posted: NfjDate,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSalary {
    currency: String,
    from: u32,
    to: u32,
    #[serde(rename = "type")]
    #[serde(default)]
    type_field: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct RawTiles {
    values: Vec<RawTileValue>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawTileValue {
    value: String,
}

/// Polymorphic date from NoFluffJobs: ISO string or integer milliseconds.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum NfjDate {
    String(String),
    Integer(i64),
}

impl Default for NfjDate {
    fn default() -> Self {
        NfjDate::Integer(0)
    }
}

impl NfjDate {
    fn to_utc(&self) -> Option<DateTime<Utc>> {
        match self {
            NfjDate::String(s) => parse_nfj_date_flexible(s),
            NfjDate::Integer(ms) => DateTime::from_timestamp_millis(*ms),
        }
    }
}

/// Clean application item after boundary normalization.
#[derive(Debug, Clone)]
struct ApplicationItem {
    posting_id: String,
    applied_date: DateTime<Utc>,
    status_history: Vec<StatusHistoryEntry>,
    offer: OfferSummary,
}

#[derive(Debug, Clone)]
struct StatusHistoryEntry {
    status: String,
    date: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct OfferSummary {
    title: String,
    budget: Option<String>,
    tags: Vec<String>,
    url: String,
    posted: Option<DateTime<Utc>>,
}

impl TryFrom<RawApplicationItem> for ApplicationItem {
    type Error = anyhow::Error;

    fn try_from(raw: RawApplicationItem) -> Result<Self, Self::Error> {
        Ok(ApplicationItem {
            posting_id: raw.posting_id,
            applied_date: raw
                .applied_date
                .to_utc()
                .ok_or_else(|| anyhow::anyhow!("invalid applied_date"))?,
            status_history: raw
                .status_history
                .into_iter()
                .map(|s| {
                    Ok(StatusHistoryEntry {
                        status: s.status,
                        date: s
                            .date
                            .to_utc()
                            .ok_or_else(|| anyhow::anyhow!("invalid status history date"))?,
                    })
                })
                .collect::<Result<_, anyhow::Error>>()?,
            offer: raw.offer.try_into()?,
        })
    }
}

impl TryFrom<RawOfferSummary> for OfferSummary {
    type Error = anyhow::Error;

    fn try_from(raw: RawOfferSummary) -> Result<Self, Self::Error> {
        let mut tags: Vec<String> = raw.tiles.values.into_iter().map(|t| t.value).collect();

        let budget = raw.salary.map(|s| {
            if !s.type_field.is_empty()
                && !tags.iter().any(|t| t.eq_ignore_ascii_case(&s.type_field))
            {
                tags.push(s.type_field.clone());
            }
            Budget {
                min: s.from,
                max: s.to,
                currency: s.currency,
                period: None,
            }
            .to_string()
        });

        let posted = raw.posted.to_utc();

        Ok(OfferSummary {
            title: raw.title,
            budget,
            tags,
            url: raw.url,
            posted,
        })
    }
}

/// Raw API response from NoFluffJobs detail endpoint.
#[derive(Debug, Clone, Deserialize)]
struct RawNoFluffJobDetail {
    #[serde(default)]
    pub basics: Basics,
    #[serde(default)]
    pub requirements: Reqs,
    #[serde(default)]
    pub details: JobDetails,
    #[serde(default)]
    pub company: Company,
    #[serde(default)]
    pub posted: Option<i64>,
    #[serde(default, rename = "expiresAt")]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub location: Location,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Basics {
    pub seniority: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobDetails {
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Company {
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Location {
    #[serde(default)]
    pub places: Vec<Place>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Place {
    #[serde(default)]
    pub city: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Reqs {
    #[serde(default)]
    pub musts: Vec<ReqItem>,
    #[serde(default)]
    pub nices: Vec<ReqItem>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub languages: Vec<Language>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Language {
    #[serde(rename = "type")]
    #[serde(default)]
    pub type_field: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReqItem {
    #[serde(default)]
    pub value: String,
    #[serde(rename = "type")]
    #[serde(default)]
    pub type_field: String,
}

#[async_trait]
impl PlatformClient for NoFluffJobsScraper {
    fn name(&self) -> &'static str {
        "nofluffjobs"
    }

    async fn fetch_with_browser(
        &self,
        browser: &Browser,
        db: &Db,
        query: &str,
        pause_ms: u64,
    ) -> Result<Vec<Job>> {
        let hosts = browser.get_page_hosts().await?;
        if !hosts.iter().any(|h| h.contains("nofluffjobs.com")) {
            bail!("NoFluffJobs requires open nofluffjobs.com tab in Brave");
        }

        self.fetch_jobs_via_browser(browser, db, query, pause_ms)
            .await
    }
}

impl NoFluffJobsScraper {
    pub fn new() -> Self {
        Self {
            config: NoFluffJobsConfig::default(),
            client: Client::builder()
                .user_agent("Mozilla/5.0 (compatible; JobSearch/1.0)")
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    pub fn with_config(config: NoFluffJobsConfig) -> Self {
        Self {
            config,
            client: Client::builder()
                .user_agent("Mozilla/5.0 (compatible; JobSearch/1.0)")
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    /// Scrape job cards from NoFluffJobs search page via browser.
    /// The website respects filters (unlike the API), so this gives accurate results.
    /// Clicks "See more offers" to load additional pages.
    pub async fn fetch_jobs_via_browser(
        &self,
        browser: &Browser,
        db: &Db,
        query: &str,
        pause_ms: u64,
    ) -> Result<Vec<Job>> {
        let search_url = self.build_search_url(query);
        let page = browser.new_tab(&search_url).await?;

        // Wait for job cards to appear
        let mut found = false;
        for _ in 0..30 {
            if page.find_element("a.posting-list-item").await.is_ok() {
                found = true;
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        if !found {
            page.close().await.ok();
            bail!("NoFluffJobs search page did not load job cards");
        }

        let total_results: Option<usize> = page
            .evaluate(GET_TOTAL_RESULTS_JS)
            .await
            .ok()
            .and_then(|v| v.into_value().ok())
            .flatten()
            .map(|n: i32| n as usize);
        if let Some(total) = total_results {
            eprintln!("  Total results: {}", total);
        }

        let mut all_jobs = Vec::new();
        let platform = Platform::NoFluffJobs;
        let mut processed_ids: HashSet<String> = HashSet::new();
        let mut checked_count = 0;

        let _guard = CursorGuard::new();

        loop {
            let cards: Vec<NofluffJobCard> = page.evaluate(SCRAPE_CARDS_JS).await?.into_value()?;

            let new_cards: Vec<_> = cards
                .into_iter()
                .filter(|c| processed_ids.insert(c.external_id.clone()))
                .collect();

            for card in &new_cards {
                checked_count += 1;
                if db.job_exists(&platform, &card.external_id).await? {
                    eprint!("\r    Progress: {:>5} {:.40}\x1B[K", checked_count, "");
                    continue;
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(pause_ms)).await;

                match self.fetch_detail(&card.external_id).await {
                    Ok(detail) => {
                        let posted = detail.posted_at;
                        let budget = card.budget.as_ref().and_then(|b| {
                            let normalized = b.replace(['\u{00a0}', '\u{2007}', '\u{202f}'], " ");
                            if normalized.trim() == "Salary Match" {
                                self.config.min_salary_eur.map(|min| {
                                    Budget {
                                        min,
                                        max: min,
                                        currency: self.config.salary_currency.clone(),
                                        period: None,
                                    }
                                    .to_string()
                                })
                            } else {
                                Budget::parse(b).map(|b| b.to_string())
                            }
                        });
                        let job = Job {
                            id: None,
                            platform,
                            external_id: card.external_id.clone(),
                            title: card.title.clone(),
                            description: None,
                            url: card.url.clone(),
                            budget,
                            tags: card.tags.clone(),
                            raw: Data::Nofluffjobs { detail },
                            created_at: posted.unwrap_or_else(chrono::Utc::now),
                            updated_at: chrono::Utc::now(),
                            liked: None,
                            note: None,
                            applied_at: None,
                        };
                        db.upsert_job(&job).await?;
                        all_jobs.push(job);
                    }
                    Err(e) => {
                        eprintln!(
                            "    Warning: failed to fetch detail for {}: {}",
                            card.external_id, e
                        );
                    }
                }

                eprint!(
                    "\r    Progress: {:>5} {:.40}\x1B[K",
                    checked_count, card.external_id
                );
            }

            if !Self::click_load_more(&page, pause_ms).await {
                break;
            }
        }

        page.close().await.ok();
        eprintln!("  Total new jobs: {}", all_jobs.len());
        Ok(all_jobs)
    }

    /// Wait for job cards to appear on search page.
    pub async fn wait_for_jobs(page: &chromiumoxide::Page) -> Result<bool> {
        for _ in 0..30 {
            let has_cards: bool = page
                .evaluate("!!document.querySelector('a.posting-list-item')")
                .await?
                .into_value()?;
            if has_cards {
                return Ok(true);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        Ok(false)
    }

    /// Scrape job cards from current page.
    pub async fn scrape_page(page: &chromiumoxide::Page) -> Result<Vec<NofluffJobCard>> {
        let cards: Vec<NofluffJobCard> = page.evaluate(SCRAPE_CARDS_JS).await?.into_value()?;
        Ok(cards)
    }

    /// Click "See more offers" button and wait for new cards. Returns true if more loaded.
    pub async fn click_load_more(page: &chromiumoxide::Page, pause_ms: u64) -> bool {
        let prev_count: i32 = page
            .evaluate(COUNT_CARDS_JS)
            .await
            .ok()
            .and_then(|v| v.into_value().ok())
            .unwrap_or(0);

        // Single JS: check, scroll, click in one go. Returns true if button was found.
        let clicked: bool = page
            .evaluate(CLICK_LOAD_MORE_JS)
            .await
            .ok()
            .and_then(|v| v.into_value().ok())
            .unwrap_or(false);

        if !clicked {
            return false;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(pause_ms)).await;

        for _ in 0..30 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            let count: i32 = page
                .evaluate(COUNT_CARDS_JS)
                .await
                .ok()
                .and_then(|v| v.into_value().ok())
                .unwrap_or(0);
            if count > prev_count {
                return true;
            }
        }
        false
    }

    /// Fetch job detail from API (no DB dependency).
    pub async fn fetch_detail(&self, job_id: &str) -> Result<NoFluffJobDetail> {
        let detail: RawNoFluffJobDetail = self
            .client
            .get(format!("{}/posting/{}", API_BASE, job_id))
            .send()
            .await?
            .json()
            .await?;

        let seniority = match detail.basics.seniority {
            Some(Value::String(ref s)) => s.clone(),
            Some(Value::Array(ref arr)) => arr
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            _ => String::new(),
        };

        let must_have = detail
            .requirements
            .musts
            .iter()
            .filter_map(|m| {
                if m.type_field == "main" {
                    Some(m.value.clone())
                } else {
                    None
                }
            })
            .collect();

        let description = html_to_md(&detail.details.description);

        let requirements = html_to_md(&detail.requirements.description);

        let nice_to_have = detail
            .requirements
            .nices
            .iter()
            .filter_map(|m| {
                if m.type_field == "main" {
                    Some(m.value.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        let languages: Vec<String> = detail
            .requirements
            .languages
            .iter()
            .filter(|l| l.type_field == "MUST")
            .map(|l| l.code.clone())
            .collect();

        let posted_at = detail
            .posted
            .and_then(chrono::DateTime::from_timestamp_millis);

        let locations: Vec<String> = detail
            .location
            .places
            .iter()
            .map(|p| p.city.clone())
            .filter(|c| !c.is_empty())
            .collect();

        let remote = if locations.iter().any(|l| l.eq_ignore_ascii_case("remote")) {
            "Remote".to_string()
        } else {
            String::new()
        };

        Ok(NoFluffJobDetail {
            company: detail.company.name,
            seniority,
            remote,
            locations,
            description,
            must_have,
            requirements,
            nice_to_have,
            offer_valid_until: detail.expires_at.unwrap_or_default(),
            languages,
            posted_at,
        })
    }

    /// Sync submitted applications from the NoFluffJobs profile page.
    pub async fn sync_applications(
        &self,
        browser: &Browser,
        db: &Db,
        pause_ms: u64,
        limit: Option<usize>,
    ) -> Result<usize> {
        let hosts = browser.get_page_hosts().await?;
        if !hosts.iter().any(|h| h.contains("nofluffjobs.com")) {
            bail!("NoFluffJobs requires open nofluffjobs.com tab in Brave");
        }

        let page = browser
            .new_tab("https://nofluffjobs.com/profile/my-applications")
            .await?;
        sleep(Duration::from_millis(pause_ms)).await;

        let per_page = 20i32;
        let mut page_num = 1i32;
        let mut all_items: Vec<ApplicationItem> = Vec::new();

        loop {
            let js = FETCH_APPLICATIONS_JS
                .replace("__PAGE__", &page_num.to_string())
                .replace("__LIMIT__", &per_page.to_string());

            let raw: Value = page.evaluate(js.as_str()).await?.into_value()?;
            if let Some(err) = raw.get("error") {
                page.close().await.ok();
                bail!("applications fetch error: {}", err);
            }

            let res: RawApplicationsResponse = serde_json::from_value(raw)?;
            for raw_item in res.items {
                match raw_item.try_into() {
                    Ok(item) => all_items.push(item),
                    Err(e) => eprintln!("  Warning: skipping malformed application item: {}", e),
                }
            }

            if !res.has_next {
                break;
            }
            page_num += 1;
            sleep(Duration::from_millis(pause_ms)).await;
        }
        page.close().await.ok();

        // Deduplicate by postingId, keep the latest application.
        let mut latest_by_posting: HashMap<String, ApplicationItem> = HashMap::new();
        for item in all_items {
            match latest_by_posting.get_mut(&item.posting_id) {
                Some(existing) => {
                    if item.applied_date > existing.applied_date {
                        *existing = item;
                    }
                }
                None => {
                    latest_by_posting.insert(item.posting_id.clone(), item);
                }
            }
        }

        let max = limit.unwrap_or(usize::MAX);
        let mut synced = 0usize;
        let total = min(max, latest_by_posting.len());

        let _guard = CursorGuard::new();
        for item in latest_by_posting.into_values() {
            if synced >= max {
                break;
            }

            if db
                .job_exists(&Platform::NoFluffJobs, &item.posting_id)
                .await?
            {
                continue;
            }

            sleep(Duration::from_millis(pause_ms)).await;

            let slug = item.offer.url.trim_start_matches('/');
            let slug = slug.strip_prefix("job/").unwrap_or(slug);
            let url = format!("https://nofluffjobs.com/job/{}", slug);

            let detail = match self.fetch_detail(slug).await {
                Ok(d) => d,
                Err(e) => {
                    eprintln!(
                        "  Warning: failed to fetch detail for {}: {}",
                        item.offer.title, e
                    );
                    continue;
                }
            };

            let created_at = detail
                .posted_at
                .or(item.offer.posted)
                .unwrap_or(item.applied_date);

            let budget = item.offer.budget.clone();
            let tags = item.offer.tags.clone();

            let job = Job {
                id: None,
                platform: Platform::NoFluffJobs,
                external_id: item.posting_id.clone(),
                title: item.offer.title.clone(),
                description: Some(detail.description.clone()).filter(|d| !d.is_empty()),
                url,
                budget,
                tags,
                raw: Data::Nofluffjobs { detail },
                created_at,
                updated_at: Utc::now(),
                liked: None,
                note: None,
                applied_at: None,
            };

            let job_id = db.upsert_job(&job).await?;
            let applied_at = applied_at_for(&item);
            db.set_applied(job_id, None, applied_at).await?;

            synced += 1;
            eprint!("\r  Progress {}/{}: {:.40}", synced, total, job.title);
        }
        eprintln!();

        Ok(synced)
    }

    fn build_criteria(&self, query: &str) -> String {
        let mut parts: Vec<String> = Vec::new();

        if let Some(emp) = &self.config.employment {
            parts.push(format!("employment={}", emp));
        }
        if let Some(salary) = self.config.min_salary_eur {
            parts.push(format!("salary>eur{}m", salary));
        }
        if let Some(lang) = &self.config.language {
            parts.push(format!("jobLanguage={}", lang));
        }
        if !query.is_empty() {
            parts.push(format!("keyword={}", query));
        }

        parts.join(" ")
    }

    pub fn build_search_url(&self, query: &str) -> String {
        let criteria = self.build_criteria(query);
        format!(
            "https://nofluffjobs.com/{}?criteria={}&sort=newest",
            self.config.path,
            urlencoding::encode(&criteria)
        )
    }
}

impl Default for NoFluffJobsScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct NoFluffJobsConfig {
    pub path: String,
    pub min_salary_eur: Option<u32>,
    pub employment: Option<String>,
    pub language: Option<String>,
    pub salary_currency: String,
}

impl Default for NoFluffJobsConfig {
    fn default() -> Self {
        Self {
            path: "remote".to_string(),
            min_salary_eur: None,
            employment: None,
            language: None,
            salary_currency: "EUR".to_string(),
        }
    }
}

fn html_to_md(html: &str) -> String {
    html2text::from_read(html.as_bytes(), 120).unwrap_or_else(|_| html.to_string())
}

fn parse_nfj_date_flexible(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|| {
            NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f")
                .ok()
                .map(|dt| dt.and_utc())
        })
        .or_else(|| {
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .ok()
                .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
        })
}

fn applied_at_for(item: &ApplicationItem) -> DateTime<Utc> {
    item.status_history
        .iter()
        .filter(|s| s.status.eq_ignore_ascii_case("applied"))
        .map(|s| s.date)
        .min()
        .unwrap_or(item.applied_date)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_search_url_default_with_query() {
        let scraper = NoFluffJobsScraper::new();
        let url = scraper.build_search_url("rust");
        assert_eq!(
            url,
            "https://nofluffjobs.com/remote?criteria=keyword%3Drust&sort=newest"
        );
    }

    #[test]
    fn test_build_search_url_empty_query() {
        let scraper = NoFluffJobsScraper::new();
        let url = scraper.build_search_url("");
        assert_eq!(url, "https://nofluffjobs.com/remote?criteria=&sort=newest");
    }

    #[test]
    fn test_build_search_url_with_all_filters() {
        let config = NoFluffJobsConfig {
            path: "remote".to_string(),
            min_salary_eur: Some(8000),
            employment: Some("b2b".to_string()),
            language: Some("en".to_string()),
            salary_currency: "EUR".to_string(),
        };
        let scraper = NoFluffJobsScraper::with_config(config);
        let url = scraper.build_search_url("rust");
        assert_eq!(
            url,
            "https://nofluffjobs.com/remote?criteria=employment%3Db2b%20salary%3Eeur8000m%20jobLanguage%3Den%20keyword%3Drust&sort=newest"
        );
    }

    #[test]
    fn test_build_search_url_filters_no_query() {
        let config = NoFluffJobsConfig {
            path: "remote".to_string(),
            min_salary_eur: Some(8000),
            employment: Some("b2b".to_string()),
            language: Some("en".to_string()),
            salary_currency: "EUR".to_string(),
        };
        let scraper = NoFluffJobsScraper::with_config(config);
        let url = scraper.build_search_url("");
        assert_eq!(
            url,
            "https://nofluffjobs.com/remote?criteria=employment%3Db2b%20salary%3Eeur8000m%20jobLanguage%3Den&sort=newest"
        );
    }

    #[test]
    fn test_build_search_url_custom_path() {
        let config = NoFluffJobsConfig {
            path: "pl/jobs".to_string(),
            min_salary_eur: None,
            employment: None,
            language: None,
            salary_currency: "EUR".to_string(),
        };
        let scraper = NoFluffJobsScraper::with_config(config);
        let url = scraper.build_search_url("senior");
        assert_eq!(
            url,
            "https://nofluffjobs.com/pl/jobs?criteria=keyword%3Dsenior&sort=newest"
        );
    }

    #[test]
    fn test_offer_summary_try_from_extracts_employment_type_as_tag() {
        let raw = RawOfferSummary {
            title: "Rust Dev".into(),
            salary: Some(RawSalary {
                currency: "EUR".into(),
                from: 6119,
                to: 8238,
                type_field: "b2b".into(),
            }),
            tiles: RawTiles {
                values: vec![
                    RawTileValue {
                        value: "rust".into(),
                    },
                    RawTileValue {
                        value: "backend".into(),
                    },
                ],
            },
            url: "rust-dev-acme".into(),
            posted: NfjDate::Integer(0),
        };
        let offer: OfferSummary = raw.try_into().unwrap();
        assert_eq!(offer.budget, Some("6119 - 8238 EUR".into()));
        assert!(offer.tags.contains(&"b2b".into()));
        assert!(offer.tags.contains(&"rust".into()));
        assert!(offer.tags.contains(&"backend".into()));
    }

    #[test]
    fn test_offer_summary_try_from_ignores_empty_salary_type() {
        let raw = RawOfferSummary {
            title: "Dev".into(),
            salary: Some(RawSalary {
                currency: "EUR".into(),
                from: 100,
                to: 200,
                type_field: "".into(),
            }),
            tiles: RawTiles { values: vec![] },
            url: "dev".into(),
            posted: NfjDate::Integer(0),
        };
        let offer: OfferSummary = raw.try_into().unwrap();
        assert_eq!(offer.budget, Some("100 - 200 EUR".into()));
        assert!(!offer.tags.contains(&"".into()));
    }

    #[test]
    fn test_application_item_try_from_parses_integer_date() {
        let raw = RawApplicationItem {
            posting_id: "ABC123".into(),
            applied_date: NfjDate::Integer(1_777_628_094_466),
            status_history: vec![],
            offer: RawOfferSummary {
                title: "Rust".into(),
                salary: None,
                tiles: RawTiles { values: vec![] },
                url: "rust".into(),
                posted: NfjDate::Integer(0),
            },
        };
        let item: ApplicationItem = raw.try_into().unwrap();
        assert_eq!(item.posting_id, "ABC123");
        assert_eq!(
            item.applied_date,
            DateTime::from_timestamp_millis(1_777_628_094_466).unwrap()
        );
    }
}
