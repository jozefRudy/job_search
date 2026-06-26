use crate::browser::{BrowserExt, host_of, wait_for, wait_for_element};
use crate::db::Db;
use crate::language::LanguageService;
use crate::models::{Data, Job, NoFluffJobDetail, Platform, Rating, classify_language};
use crate::platforms::{FetchState, PlatformClient};
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
const POSTING_LIST_ITEM_JS: &str = include_str!("nofluffjobs/posting_list_item.js");
const CLICK_LOAD_MORE_JS: &str = include_str!("nofluffjobs/click_load_more.js");
const COUNT_CARDS_JS: &str = include_str!("nofluffjobs/count_cards.js");
const GET_TOTAL_RESULTS_JS: &str = include_str!("nofluffjobs/get_total_results.js");
const FETCH_APPLICATIONS_JS: &str = include_str!("nofluffjobs/fetch_applications.js");

/// Card scraped from `NoFluffJobs` search page DOM.
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
    lang: LanguageService,
}

const API_BASE: &str = "https://nofluffjobs.com/api";

/// Raw API response from `NoFluffJobs` /candidates/my-applications endpoint.
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
    employment_type: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct RawTiles {
    values: Vec<RawTileValue>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawTileValue {
    value: String,
}

/// Polymorphic date from `NoFluffJobs`: ISO string or integer milliseconds.
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
    employment_type: Option<String>,
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
        let tags: Vec<String> = raw.tiles.values.into_iter().map(|t| t.value).collect();

        let employment_type = raw.salary.as_ref().and_then(|s| {
            if s.employment_type.is_empty() {
                None
            } else {
                Some(s.employment_type.clone())
            }
        });

        let budget = raw.salary.map(|s| {
            let text = format!("{} - {} {}", s.from, s.to, s.currency);
            crate::extractors::budget::parse_nofluff_budget(&text)
                .map(|b| b.to_string())
                .unwrap_or(text)
        });

        let posted = raw.posted.to_utc();

        Ok(OfferSummary {
            title: raw.title,
            budget,
            tags,
            url: raw.url,
            posted,
            employment_type,
        })
    }
}

/// Raw API response from `NoFluffJobs` detail endpoint.
///
/// Active postings return detail fields at the top level; expired postings wrap
/// them under `jobOffer`. We normalize both shapes at the boundary.
#[derive(Debug, Clone, Deserialize, Default)]
struct RawPostingEnvelope {
    #[serde(default, rename = "jobOffer")]
    job_offer: Option<RawNoFluffJobDetail>,
    #[serde(default)]
    basics: Basics,
    #[serde(default)]
    requirements: Reqs,
    #[serde(default)]
    details: JobDetails,
    #[serde(default)]
    company: Company,
    #[serde(default)]
    posted: Option<i64>,
    #[serde(default, rename = "expiresAt")]
    expires_at: Option<String>,
    #[serde(default)]
    location: Location,
}

#[derive(Debug, Clone, Deserialize, Default)]
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

impl TryFrom<RawPostingEnvelope> for NoFluffJobDetail {
    type Error = anyhow::Error;

    fn try_from(envelope: RawPostingEnvelope) -> Result<Self, Self::Error> {
        let raw = envelope.job_offer.unwrap_or(RawNoFluffJobDetail {
            basics: envelope.basics,
            requirements: envelope.requirements,
            details: envelope.details,
            company: envelope.company,
            posted: envelope.posted,
            expires_at: envelope.expires_at,
            location: envelope.location,
        });

        let seniority = match raw.basics.seniority {
            Some(Value::String(ref s)) => s.clone(),
            Some(Value::Array(ref arr)) => arr
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            _ => String::new(),
        };

        let must_have = raw
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

        let description = html_to_md(&raw.details.description);

        let requirements = html_to_md(&raw.requirements.description);

        let nice_to_have = raw
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

        let languages: Vec<String> = raw
            .requirements
            .languages
            .iter()
            .filter(|l| l.type_field == "MUST")
            .map(|l| l.code.clone())
            .collect();

        let posted_at = raw
            .posted
            .and_then(chrono::DateTime::from_timestamp_millis)
            .unwrap_or_else(Utc::now);

        let locations: Vec<String> = raw
            .location
            .places
            .iter()
            .map(|p| p.city.clone())
            .filter(|c| !c.is_empty())
            .collect();

        Ok(NoFluffJobDetail {
            company: raw.company.name,
            seniority,
            locations,
            description,
            must_have,
            requirements,
            nice_to_have,
            offer_valid_until: raw.expires_at.unwrap_or_default(),
            languages,
            posted_at,
            employment_type: None,
        })
    }
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
    ) -> Result<FetchState> {
        let page_hosts: Vec<_> = browser
            .get_page_urls()
            .await?
            .into_iter()
            .filter_map(|u| host_of(&u))
            .collect();
        if !page_hosts.iter().any(|h| h.contains("nofluffjobs.com")) {
            bail!("NoFluffJobs requires open nofluffjobs.com tab in Brave");
        }

        self.fetch_jobs_via_browser(browser, db, query, pause_ms)
            .await
    }

    async fn sync_applications(
        &self,
        browser: &Browser,
        db: &Db,
        pause_ms: u64,
        limit: Option<usize>,
    ) -> Result<FetchState> {
        NoFluffJobsScraper::sync_applications(self, browser, db, pause_ms, limit).await
    }
}

impl NoFluffJobsScraper {
    #[must_use]
    pub fn new(lang: LanguageService) -> Self {
        Self {
            config: NoFluffJobsConfig::default(),
            client: Client::builder()
                .user_agent("Mozilla/5.0 (compatible; JobSearch/1.0)")
                .build()
                .unwrap_or_else(|_| Client::new()),
            lang,
        }
    }

    #[must_use]
    pub fn with_config(config: NoFluffJobsConfig, lang: LanguageService) -> Self {
        Self {
            config,
            client: Client::builder()
                .user_agent("Mozilla/5.0 (compatible; JobSearch/1.0)")
                .build()
                .unwrap_or_else(|_| Client::new()),
            lang,
        }
    }

    async fn set_currency_cookie(&self, browser: &Browser) -> Result<()> {
        browser
            .set_cookie(
                "nfj_ui_settings_currency",
                &self.config.salary_currency.to_lowercase(),
                ".nofluffjobs.com",
            )
            .await
    }

    /// Scrape job cards from `NoFluffJobs` search page via browser.
    /// The website respects filters (unlike the API), so this gives accurate results.
    /// Clicks "See more offers" to load additional pages.
    pub async fn fetch_jobs_via_browser(
        &self,
        browser: &Browser,
        db: &Db,
        query: &str,
        pause_ms: u64,
    ) -> Result<FetchState> {
        let search_url = self.build_search_url(query);
        self.set_currency_cookie(browser).await?;
        let page = browser.new_tab(&search_url).await?;

        // Wait for job cards to appear
        if !wait_for_element(&page, &["a.posting-list-item"], None, None).await? {
            page.close().await.ok();
            bail!("NoFluffJobs search page did not load job cards");
        }

        let total_results: Option<usize> = page
            .evaluate(GET_TOTAL_RESULTS_JS)
            .await
            .ok()
            .and_then(|v| v.into_value().ok())
            .flatten()
            .and_then(|n: i32| usize::try_from(n).ok());

        let mut all_jobs = Vec::new();
        let platform = Platform::NoFluffJobs;
        let mut processed_ids: HashSet<String> = HashSet::new();
        let mut state = FetchState::new();

        let _guard = CursorGuard::new();

        loop {
            let cards: Vec<NofluffJobCard> = page.evaluate(SCRAPE_CARDS_JS).await?.into_value()?;

            let new_cards: Vec<_> = cards
                .into_iter()
                .filter(|c| processed_ids.insert(c.external_id.clone()))
                .collect();

            for card in &new_cards {
                if db
                    .find_job_id(&platform, &card.external_id)
                    .await?
                    .is_some()
                {
                    state.inc_existing();
                    eprint!("{}", state.progress_line(total_results, ""));
                    continue;
                }

                sleep(Duration::from_millis(pause_ms)).await;

                match self.fetch_detail(&card.external_id).await {
                    Ok(detail) => {
                        let posted = detail.posted_at;
                        let budget = card.budget.as_ref().and_then(|b| {
                            let normalized = b.replace(['\u{00a0}', '\u{2007}', '\u{202f}'], " ");
                            if normalized.trim() == "Salary Match" {
                                self.config.min_salary_eur.map(|min| {
                                    let text = format!("{} {}", min, self.config.salary_currency);
                                    crate::extractors::budget::parse_nofluff_budget(&text)
                                        .map(|b| b.to_string())
                                        .unwrap_or(text)
                                })
                            } else {
                                crate::extractors::budget::parse_nofluff_budget(b)
                                    .map(|b| b.to_string())
                            }
                        });
                        let job = Job {
                            id: 0,
                            platform,
                            external_id: card.external_id.clone(),
                            title: card.title.clone(),
                            description: None,
                            url: card.url.clone(),
                            budget,
                            tags: card.tags.clone(),
                            raw: Data::Nofluffjobs { detail },
                            company: None,
                            created_at: posted,
                            updated_at: chrono::Utc::now(),
                            rating: Rating::Neutral,
                            note: None,
                            applied_at: None,
                            remote: true,
                            is_english: true,
                        };
                        let is_english = classify_language(&self.lang, &job).await?;
                        let job = Job { is_english, ..job };
                        db.upsert_job(&job).await?;
                        state.inc_new();
                        all_jobs.push(job);
                    }
                    Err(e) => {
                        eprintln!(
                            "    Warning: failed to fetch detail for {}: {}",
                            card.external_id, e
                        );
                    }
                }

                eprint!("{}", state.progress_line(total_results, &card.external_id));
            }

            if !Self::click_load_more(&page, pause_ms).await {
                break;
            }
        }

        page.close().await.ok();
        Ok(state)
    }

    /// Wait for job cards to appear on search page.
    pub async fn wait_for_jobs(page: &chromiumoxide::Page) -> Result<bool> {
        crate::browser::wait_for_with_challenge_recovery(
            page,
            POSTING_LIST_ITEM_JS,
            None,
            None,
            None,
            None,
        )
        .await
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

        sleep(Duration::from_millis(pause_ms)).await;

        wait_for(
            || async {
                let count: i32 = page
                    .evaluate(COUNT_CARDS_JS)
                    .await
                    .ok()
                    .and_then(|v| v.into_value().ok())
                    .unwrap_or(0);
                Ok(count > prev_count)
            },
            None,
            None,
        )
        .await
        .unwrap_or(false)
    }

    /// Fetch job detail from API (no DB dependency).
    pub async fn fetch_detail(&self, job_id: &str) -> Result<NoFluffJobDetail> {
        let url = format!("{API_BASE}/posting/{job_id}");
        let envelope: RawPostingEnvelope = self
            .client
            .get(&url)
            .query(&[
                ("salaryCurrency", self.config.salary_currency.as_str()),
                ("salaryPeriod", "month"),
            ])
            .send()
            .await?
            .json()
            .await?;

        envelope.try_into()
    }

    /// Sync submitted applications from the `NoFluffJobs` profile page.
    pub async fn sync_applications(
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
        if !page_hosts.iter().any(|h| h.contains("nofluffjobs.com")) {
            bail!("NoFluffJobs requires open nofluffjobs.com tab in Brave");
        }

        self.set_currency_cookie(browser).await?;
        let page = browser
            .new_tab("https://nofluffjobs.com/profile/my-applications")
            .await?;
        sleep(Duration::from_millis(pause_ms)).await;

        let all_items = self.fetch_application_items(&page, pause_ms).await?;
        page.close().await.ok();

        let deduped = dedupe_latest_by_posting(all_items);
        let max = limit.unwrap_or(usize::MAX);
        let mut state = FetchState::new();
        let total = min(max, deduped.len());

        let _guard = CursorGuard::new();
        for item in deduped {
            if state.checked() >= max {
                break;
            }

            let job_id = match self.upsert_application_job(db, &item, pause_ms).await {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("  Warning: failed to sync {}: {e}", item.offer.title);
                    continue;
                }
            };
            let Some(job_id) = job_id else { continue };

            let stored_applied = db
                .get_job(job_id)
                .await?
                .and_then(|j| j.applied_at)
                .is_some();

            let label = &item.offer.title;

            if stored_applied {
                state.inc_existing();
                eprint!("{}", state.progress_line(Some(total), label));
                continue;
            }

            db.set_applied(job_id, None, applied_at_for(&item)).await?;
            state.inc_new();
            eprint!("{}", state.progress_line(Some(total), label));
        }

        Ok(state)
    }

    async fn fetch_application_items(
        &self,
        page: &chromiumoxide::Page,
        pause_ms: u64,
    ) -> Result<Vec<ApplicationItem>> {
        let per_page = 20i32;
        let mut page_num = 0i32;
        let mut all_items = Vec::new();

        loop {
            let js = FETCH_APPLICATIONS_JS
                .replace("__PAGE__", &page_num.to_string())
                .replace("__LIMIT__", &per_page.to_string());

            let raw: Value = page.evaluate(js.as_str()).await?.into_value()?;
            if let Some(err) = raw.get("error") {
                bail!("applications fetch error: {err}");
            }

            let res: RawApplicationsResponse = serde_json::from_value(raw)?;
            for raw_item in res.items {
                match raw_item.try_into() {
                    Ok(item) => all_items.push(item),
                    Err(e) => eprintln!("  Warning: skipping malformed application item: {e}"),
                }
            }

            if !res.has_next {
                break;
            }
            page_num += 1;
            sleep(Duration::from_millis(pause_ms)).await;
        }

        Ok(all_items)
    }

    async fn upsert_application_job(
        &self,
        db: &Db,
        item: &ApplicationItem,
        pause_ms: u64,
    ) -> Result<Option<i64>> {
        let slug = item.offer.url.trim().to_lowercase();
        if slug.is_empty() || slug.contains('/') || slug.contains('?') {
            eprintln!(
                "  Warning: application item has invalid job slug for {}",
                item.offer.title
            );
            return Ok(None);
        }
        let url = format!("https://nofluffjobs.com/job/{slug}");
        let external_id = slug.clone();

        if let Some(job_id) = db.find_job_id(&Platform::NoFluffJobs, &external_id).await? {
            return Ok(Some(job_id));
        }

        sleep(Duration::from_millis(pause_ms)).await;

        let mut detail = match self.fetch_detail(&slug).await {
            Ok(d) => d,
            Err(e) => {
                eprintln!(
                    "  Warning: failed to fetch detail for {}: {}",
                    item.offer.title, e
                );
                return Ok(None);
            }
        };

        detail.employment_type = item.offer.employment_type.clone();
        if let Some(posted) = item.offer.posted {
            detail.posted_at = posted;
        }

        let applied_at = applied_at_for(item);
        let mut created_at = detail.posted_at;
        if created_at > applied_at {
            created_at = applied_at;
        }

        let description = Some(detail.description.clone()).filter(|d| !d.is_empty());
        let job = Job {
            id: 0,
            platform: Platform::NoFluffJobs,
            external_id,
            title: item.offer.title.clone(),
            description,
            url,
            budget: item.offer.budget.clone(),
            tags: item.offer.tags.clone(),
            raw: Data::Nofluffjobs { detail },
            company: None,
            created_at,
            updated_at: Utc::now(),
            rating: Rating::Neutral,
            note: None,
            applied_at: None,
            remote: true,
            is_english: true,
        };
        let is_english = classify_language(&self.lang, &job).await?;
        let job = Job { is_english, ..job };
        Ok(Some(db.upsert_job(&job).await?))
    }

    fn build_criteria(&self, query: &str) -> String {
        let mut parts: Vec<String> = Vec::new();

        if let Some(emp) = &self.config.employment {
            parts.push(format!("employment={emp}"));
        }
        if let Some(salary) = self.config.min_salary_eur {
            parts.push(format!("salary>eur{salary}m"));
        }
        if let Some(lang) = &self.config.language {
            parts.push(format!("jobLanguage={lang}"));
        }
        if !query.is_empty() {
            parts.push(format!("keyword={query}"));
        }

        parts.join(" ")
    }

    #[must_use]
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
        Self::new(LanguageService::new())
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

fn dedupe_latest_by_posting(items: Vec<ApplicationItem>) -> Vec<ApplicationItem> {
    let mut latest_by_posting: HashMap<String, ApplicationItem> = HashMap::new();
    for item in items {
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
    latest_by_posting.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_search_url_default_with_query() {
        let scraper = NoFluffJobsScraper::new(LanguageService::new());
        let url = scraper.build_search_url("rust");
        assert_eq!(
            url,
            "https://nofluffjobs.com/remote?criteria=keyword%3Drust&sort=newest"
        );
    }

    #[test]
    fn test_build_search_url_empty_query() {
        let scraper = NoFluffJobsScraper::new(LanguageService::new());
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
        let scraper = NoFluffJobsScraper::with_config(config, LanguageService::new());
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
        let scraper = NoFluffJobsScraper::with_config(config, LanguageService::new());
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
        let scraper = NoFluffJobsScraper::with_config(config, LanguageService::new());
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
                employment_type: "b2b".into(),
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
        assert_eq!(offer.budget, Some("6119 - 8238 EUR/mo".into()));
        assert_eq!(offer.employment_type, Some("b2b".into()));
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
                employment_type: String::new(),
            }),
            tiles: RawTiles { values: vec![] },
            url: "dev".into(),
            posted: NfjDate::Integer(0),
        };
        let offer: OfferSummary = raw.try_into().unwrap();
        assert_eq!(offer.budget, Some("100 - 200 EUR/mo".into()));
        assert!(!offer.tags.contains(&String::new()));
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

    #[test]
    fn test_expired_detail_joboffer_shape_parses() {
        let json = include_str!("../../tests/fixtures/nofluffjobs_expired_detail.json");
        let envelope: RawPostingEnvelope = serde_json::from_str(json).expect("deserialize fixture");
        let detail: NoFluffJobDetail = envelope.try_into().expect("convert to detail");

        assert_eq!(detail.company, "ServiceTitan");
        assert_eq!(detail.seniority, "Expert");

        assert!(detail.description.contains("Flexibility"));
        assert!(detail.requirements.contains("Ready to be a Titan"));
        assert!(detail.must_have.contains(&".NET".to_string()));
        assert!(detail.nice_to_have.contains("React"));
        assert_eq!(detail.languages, vec!["en"]);
        assert!(detail.posted_at <= Utc::now() - chrono::Duration::minutes(1));
    }
}
