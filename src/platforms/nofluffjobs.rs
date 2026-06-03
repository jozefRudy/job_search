use crate::browser::BrowserExt;
use crate::db::Db;
use crate::models::{Data, Job, JobStatus, NoFluffJobDetail, Platform, Reaction};
use crate::platforms::PlatformClient;
use anyhow::{Result, bail};
use async_trait::async_trait;
use chromiumoxide::browser::Browser;
use chrono::{TimeZone, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub struct NoFluffJobsScraper {
    config: NoFluffJobsConfig,
    client: Client,
}

const API_BASE: &str = "https://nofluffjobs.com/api";
const LIST_ENDPOINT: &str = "/joboffers/main";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiPosting {
    pub id: String,
    pub name: String,
    pub title: String,
    pub url: String,
    pub posted: i64,
    pub renewed: Option<i64>,
    pub seniority: Seniority,
    pub technology: Option<Technology>,
    #[serde(default)]
    pub fully_remote: bool,
    pub salary: Option<Salary>,
    pub regions: Vec<String>,
    #[serde(default)]
    pub flavors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Seniority {
    Single(String),
    Multiple(Vec<String>),
}

impl Seniority {
    pub fn as_string(&self) -> String {
        match self {
            Seniority::Single(s) => s.clone(),
            Seniority::Multiple(v) => v.join(", "),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Technology {
    Single(String),
    Multiple(Vec<String>),
    None,
}

impl Technology {
    pub fn as_vec(&self) -> Vec<String> {
        match self {
            Technology::Single(s) => vec![s.clone()],
            Technology::Multiple(v) => v.clone(),
            Technology::None => vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Salary {
    pub from: f64,
    pub to: f64,
    #[serde(rename = "type")]
    pub salary_type: String,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub country: Country,
    pub city: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Country {
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse {
    pub postings: Vec<ApiPosting>,
    #[serde(rename = "totalCount")]
    pub total_count: i64,
    #[serde(rename = "totalPages")]
    pub total_pages: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailResponse {
    #[serde(default)]
    pub basics: Basics,
    #[serde(default)]
    pub requirements: Reqs,
    #[serde(default)]
    pub details: JobDetails,
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

        // Use API instead of browser scraping
        self.fetch_jobs(db, query, pause_ms).await
    }

    async fn react(&self, job: &Job, action: Reaction) -> Result<()> {
        self.react_via_browser(job, action).await
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

    /// Fetch single page of job postings from API (no DB dependency).
    /// Testable without DB or network mocking.
    pub async fn fetch_page(&self, query: &str, page: i64) -> Result<ListResponse> {
        let criteria = self.build_criteria(query);
        let response: ListResponse = self
            .client
            .get(format!("{}{}", API_BASE, LIST_ENDPOINT))
            .query(&[
                ("criteria", &criteria),
                ("salaryCurrency", &self.config.salary_currency),
                ("salaryPeriod", &"month".to_string()),
                ("pageNumber", &page.to_string()),
            ])
            .send()
            .await?
            .json()
            .await?;
        Ok(response)
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

        Ok(NoFluffJobDetail {
            company: String::new(),
            seniority,
            remote: String::new(),
            locations: vec![],
            must_have,
            requirements,
            offer_description,
            offer_valid_until: String::new(),
            languages,
        })
    }

    /// Fetch jobs and store in DB. Stops when finds already-existing job.
    pub async fn fetch_jobs(&self, db: &Db, query: &str, pause_ms: u64) -> Result<Vec<Job>> {
        self.fetch_jobs_with_limit(db, query, pause_ms, None).await
    }

    /// Check if posting matches config filters (client-side, since API ignores criteria).
    fn matches_filters(&self, posting: &ApiPosting) -> bool {
        if let Some(min) = self.config.min_salary_eur {
            let upper = posting.salary.as_ref().map(|s| s.to).unwrap_or(0.0);
            if (upper as u32) < min {
                return false;
            }
        }
        if let Some(ref emp) = self.config.employment {
            let posting_type = posting.salary.as_ref().map(|s| &s.salary_type);
            if posting_type != Some(emp) {
                return false;
            }
        }
        true
    }

    /// Fetch jobs with configurable page limit (useful for testing).
    pub async fn fetch_jobs_with_limit(
        &self,
        db: &Db,
        query: &str,
        pause_ms: u64,
        max_pages: Option<i64>,
    ) -> Result<Vec<Job>> {
        let mut all_jobs = Vec::new();
        let platform = Platform::NoFluffJobs;
        let mut page = 0;
        let mut seen_this_run: HashSet<(String, String)> = HashSet::new();

        loop {
            if max_pages.is_some_and(|m| page >= m) {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(pause_ms)).await;

            let response = self.fetch_page(query, page).await?;

            eprintln!(
                "  Page {}: {} jobs (total: {})",
                page,
                response.postings.len(),
                response.total_count
            );

            let mut stopped = false;

            for posting in &response.postings {
                let dedup_key = (posting.name.clone(), posting.title.clone());
                if !seen_this_run.insert(dedup_key) {
                    continue;
                }

                if db.job_exists(&platform, &posting.id).await? {
                    eprintln!(
                        "  Stopping: '{}' already in DB ({})",
                        posting.title, posting.id
                    );
                    stopped = true;
                    continue;
                }

                if !self.matches_filters(posting) {
                    continue;
                }

                match self.fetch_detail(&posting.id).await {
                    Ok(detail) => {
                        let posted_at = Utc.timestamp_millis_opt(posting.posted).single();
                        let tags = posting
                            .technology
                            .as_ref()
                            .map(|t| t.as_vec())
                            .unwrap_or_default();
                        let budget = posting
                            .salary
                            .as_ref()
                            .map(|s| format!("{} - {} {}", s.from as i32, s.to as i32, s.currency));

                        let job = Job {
                            id: None,
                            platform,
                            external_id: posting.id.clone(),
                            title: posting.title.clone(),
                            description: None,
                            url: format!("https://nofluffjobs.com/job/{}", posting.url),
                            posted_at,
                            budget,
                            tags,
                            raw: Data::Nofluffjobs { detail },
                            status: JobStatus::New,
                            created_at: None,
                            updated_at: None,
                        };

                        db.upsert_job(&job).await?;
                        all_jobs.push(job);
                    }
                    Err(e) => {
                        eprintln!(
                            "    Warning: failed to fetch detail for {}: {}",
                            posting.id, e
                        );
                    }
                }
            }

            if stopped {
                break;
            }

            page += 1;
            if page >= response.total_pages {
                break;
            }
        }

        eprintln!("  Total new jobs: {}", all_jobs.len());
        Ok(all_jobs)
    }

    /// Open job detail page in browser for reacting
    pub async fn react_via_browser(&self, job: &Job, action: Reaction) -> Result<()> {
        match action {
            Reaction::Save | Reaction::Apply => {
                eprintln!(
                    "Open this URL in browser and {}: {}",
                    if action == Reaction::Save {
                        "save"
                    } else {
                        "apply"
                    },
                    job.url
                );
                Ok(())
            }
            Reaction::Hide => Ok(()),
        }
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
            "https://nofluffjobs.com/{}?criteria={}",
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
            "https://nofluffjobs.com/remote?criteria=keyword%3Drust"
        );
    }

    #[test]
    fn test_build_search_url_empty_query() {
        let scraper = NoFluffJobsScraper::new();
        let url = scraper.build_search_url("");
        assert_eq!(url, "https://nofluffjobs.com/remote?criteria=");
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
            "https://nofluffjobs.com/remote?criteria=employment%3Db2b%20salary%3Eeur8000m%20jobLanguage%3Den%20keyword%3Drust"
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
            "https://nofluffjobs.com/remote?criteria=employment%3Db2b%20salary%3Eeur8000m%20jobLanguage%3Den"
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
            "https://nofluffjobs.com/pl/jobs?criteria=keyword%3Dsenior"
        );
    }

    #[test]
    fn test_technology_as_vec() {
        assert_eq!(
            Technology::Single("Rust".to_string()).as_vec(),
            vec!["Rust"]
        );
        assert_eq!(
            Technology::Multiple(vec!["Rust".to_string(), "Python".to_string()]).as_vec(),
            vec!["Rust", "Python"]
        );
        assert!(Technology::None.as_vec().is_empty());
    }
}
