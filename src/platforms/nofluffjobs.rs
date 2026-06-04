use crate::browser::BrowserExt;
use crate::db::Db;
use crate::models::{Budget, Data, Job, NoFluffJobDetail, Platform};
use crate::platforms::PlatformClient;
use anyhow::{Result, bail};
use async_trait::async_trait;
use chromiumoxide::browser::Browser;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailResponse {
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

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let total_results: Option<usize> = page
            .evaluate(super::nofluffjobs_js::GET_TOTAL_RESULTS)
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

        eprint!("\x1B[?25l"); // hide cursor

        loop {
            let cards: Vec<NofluffJobCard> = page
                .evaluate(super::nofluffjobs_js::SCRAPE_CARDS)
                .await?
                .into_value()?;

            let new_cards: Vec<_> = cards
                .into_iter()
                .filter(|c| processed_ids.insert(c.external_id.clone()))
                .collect();

            for card in &new_cards {
                checked_count += 1;
                if db.job_exists(&platform, &card.external_id).await? {
                    eprint!("\r    Progress: {:>5}", checked_count);
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
                            created_at: posted,
                            updated_at: None,
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
                    checked_count, &card.external_id
                );
            }

            if !Self::click_load_more(&page, pause_ms).await {
                break;
            }
        }

        eprintln!("\x1B[?25h"); // show cursor
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
        let cards: Vec<NofluffJobCard> = page
            .evaluate(super::nofluffjobs_js::SCRAPE_CARDS)
            .await?
            .into_value()?;
        Ok(cards)
    }

    /// Click "See more offers" button and wait for new cards. Returns true if more loaded.
    pub async fn click_load_more(page: &chromiumoxide::Page, pause_ms: u64) -> bool {
        let prev_count: i32 = page
            .evaluate(super::nofluffjobs_js::COUNT_CARDS)
            .await
            .ok()
            .and_then(|v| v.into_value().ok())
            .unwrap_or(0);

        // Single JS: check, scroll, click in one go. Returns true if button was found.
        let clicked: bool = page
            .evaluate(super::nofluffjobs_js::CLICK_LOAD_MORE)
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
                .evaluate(super::nofluffjobs_js::COUNT_CARDS)
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
        let detail: DetailResponse = self
            .client
            .get(format!("{}/posting/{}", API_BASE, job_id))
            .send()
            .await?
            .json()
            .await?;

        let seniority = detail
            .basics
            .seniority
            .as_ref()
            .and_then(|s| s.as_str().map(String::from))
            .unwrap_or_default();

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

        let requirements = detail.requirements.description.clone();

        let offer_description = detail
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
            must_have,
            requirements,
            offer_description,
            offer_valid_until: detail.expires_at.unwrap_or_default(),
            languages,
            posted_at,
        })
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
}
