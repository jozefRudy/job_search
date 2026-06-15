use crate::browser::{BrowserExt, host_of, wait_for_element};
use crate::db::Db;
use crate::models::{Budget, Data, EfinancialcareersJobDetail, Job, Platform, parse_relative_time};
use crate::platforms::PlatformClient;
use crate::term::CursorGuard;
use anyhow::{Result, bail};
use async_trait::async_trait;
use chromiumoxide::browser::Browser;
use chrono::DateTime;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio::time::sleep;

const SCRAPE_CARDS_JS: &str = include_str!("efinancialcareers/scrape_cards.js");
const CLICK_SHOW_MORE_JS: &str = include_str!("efinancialcareers/click_show_more.js");
const SCRAPE_TOTAL_JS: &str = include_str!("efinancialcareers/scrape_total.js");
const EXTRACT_DETAIL_JS: &str = include_str!("efinancialcareers/extract_detail.js");
const FETCH_APPLICATIONS_JS: &str = include_str!("efinancialcareers/fetch_applications.js");
const EXTRACT_AUTH_JS: &str = include_str!("efinancialcareers/extract_auth.js");

const BATCH_CHUNK_SIZE: usize = 100;

/// Card scraped from eFinancialCareers search page DOM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EfinancialcareersJobCard {
    pub external_id: String,
    pub title: String,
    pub url: String,
    pub salary: String,
    #[serde(default)]
    pub posted_at_text: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct RawApplicationItem {
    #[serde(default)]
    internal_job_id: String,
    external_id: String,
    title: String,
    url: String,
    #[serde(default)]
    salary: String,
    #[serde(default)]
    company: String,
    #[serde(default)]
    location: String,
    #[serde(default)]
    employment_type: String,
    #[serde(default)]
    applied_at_text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum ApplicationsResult {
    Ok { applied: Vec<RawApplicationItem> },
    Error { error: String },
}

#[derive(Debug, Clone)]
struct ApplicationItem {
    internal_job_id: String,
    external_id: String,
    title: String,
    url: String,
    salary: String,
    company: String,
    location: String,
    employment_type: String,
    applied_at: DateTime<Utc>,
}

impl TryFrom<RawApplicationItem> for ApplicationItem {
    type Error = anyhow::Error;

    fn try_from(raw: RawApplicationItem) -> Result<Self, Self::Error> {
        if raw.external_id.is_empty() {
            bail!("job url missing id: {}", raw.url);
        }

        let applied_at = DateTime::parse_from_rfc3339(&raw.applied_at_text)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(ApplicationItem {
            internal_job_id: raw.internal_job_id,
            external_id: raw.external_id,
            title: raw.title,
            url: raw.url,
            salary: raw.salary,
            company: raw.company,
            location: raw.location,
            employment_type: raw.employment_type,
            applied_at,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
struct AuthInfo {
    token: String,
    #[serde(rename = "jobseeker_id")]
    jobseeker_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum AuthResult {
    Ok(AuthInfo),
    Error { error: String },
}

#[derive(Debug, Clone, Deserialize)]
struct BatchJob {
    id: String,
    #[serde(default)]
    description: String,
}

#[derive(Debug, Clone, Deserialize)]
struct BatchResponse {
    data: Vec<BatchJob>,
}

pub struct EfinancialcareersScraper {
    config: EfinancialcareersConfig,
}

impl EfinancialcareersScraper {
    pub fn new() -> Self {
        Self {
            config: EfinancialcareersConfig::default(),
        }
    }

    pub fn with_config(config: EfinancialcareersConfig) -> Self {
        Self { config }
    }

    pub fn build_search_url(&self, query: &str) -> String {
        let keyword = query.trim();
        // eFinancialCareers expects literal `+` for spaces in q= param.
        let encoded = keyword.replace(" ", "+");
        // Path slug: single-word or empty. Multi-word queries use q= only.
        let first_word = keyword.split_whitespace().next().unwrap_or("");
        let path_slug = if first_word.is_empty() {
            String::new()
        } else {
            format!("/{}", first_word)
        };

        format!(
            "https://www.efinancialcareers.com/jobs/remote{}?radius=50&radiusUnit=mi&pageSize=100&filters.workArrangementType={}&currencyCode={}&filters.minSalary={}&language={}&q={}&includeUnspecifiedSalary=true&enableVectorSearch=true",
            path_slug,
            self.config.work_arrangement,
            self.config.currency_code,
            self.config.min_salary,
            self.config.language,
            encoded,
        )
    }

    pub async fn wait_for_jobs(page: &chromiumoxide::Page) -> Result<bool> {
        wait_for_element(
            page,
            &[
                "efc-job-search-results",
                "efc-empty-job-search-results-wrapper",
            ],
            None,
            None,
        )
        .await
    }

    pub async fn scrape_page(page: &chromiumoxide::Page) -> Result<Vec<EfinancialcareersJobCard>> {
        let cards: Vec<EfinancialcareersJobCard> =
            page.evaluate(SCRAPE_CARDS_JS).await?.into_value()?;
        Ok(cards)
    }

    pub async fn scrape_total(page: &chromiumoxide::Page) -> Result<usize> {
        let total: Option<i64> = page.evaluate(SCRAPE_TOTAL_JS).await?.into_value()?;
        match total {
            Some(n) if n >= 0 => Ok(n as usize),
            _ => bail!("eFinancialCareers total job count not found in heading"),
        }
    }

    pub async fn click_show_more(page: &chromiumoxide::Page, pause_ms: u64) -> bool {
        let clicked: bool = page
            .evaluate(CLICK_SHOW_MORE_JS)
            .await
            .ok()
            .and_then(|v| v.into_value().ok())
            .unwrap_or(false);

        if !clicked {
            return false;
        }

        sleep(Duration::from_millis(pause_ms)).await;
        true
    }

    /// Fetch job detail from a detail page.
    pub async fn fetch_detail(
        &self,
        browser: &Browser,
        url: &str,
    ) -> Result<EfinancialcareersJobDetail> {
        let page = browser.new_tab(url).await?;

        let _ = wait_for_element(&page, &["efc-job-description"], None, None).await;

        let extracted: ExtractedDetail = page.evaluate(EXTRACT_DETAIL_JS).await?.into_value()?;

        page.close().await.ok();

        Ok(EfinancialcareersJobDetail {
            company: String::new(),
            location: String::new(),
            employment_type: String::new(),
            salary: extracted.salary,
            description: extracted.description,
            posted_at: crate::models::parse_relative_time(&extracted.posted_at_text),
        })
    }

    fn html_to_text(html: &str) -> Option<String> {
        if html.trim().is_empty() {
            return None;
        }
        let text = html2text::from_read(html.as_bytes(), 120).ok()?;
        let text = text.trim().to_string();
        if text.is_empty() { None } else { Some(text) }
    }

    fn build_job(
        &self,
        card: &EfinancialcareersJobCard,
        detail: EfinancialcareersJobDetail,
    ) -> Job {
        let created_at = detail
            .posted_at
            .unwrap_or_else(|| parse_relative_time(&card.posted_at_text).unwrap_or_else(Utc::now));
        let salary = if detail.salary.is_empty() {
            card.salary.clone()
        } else {
            detail.salary.clone()
        };
        let budget = Budget::parse(&salary, Some("year"))
            .map(|b| b.to_string())
            .or_else(|| Some(salary.clone()).filter(|b| !b.is_empty()));

        Job {
            id: None,
            platform: Platform::Efinancialcareers,
            external_id: card.external_id.clone(),
            title: card.title.clone(),
            description: Some(detail.description.clone()).filter(|d| !d.is_empty()),
            url: card.url.clone(),
            budget,
            tags: Vec::new(),
            raw: Data::Efinancialcareers {
                detail: EfinancialcareersJobDetail {
                    salary: salary.clone(),
                    ..detail
                },
            },
            created_at,
            updated_at: Utc::now(),
            liked: None,
            note: None,
            applied_at: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ExtractedDetail {
    #[serde(default)]
    description: String,
    #[serde(default)]
    salary: String,
    #[serde(default)]
    posted_at_text: String,
}

#[async_trait]
impl PlatformClient for EfinancialcareersScraper {
    fn name(&self) -> &'static str {
        "efinancialcareers"
    }

    async fn fetch_with_browser(
        &self,
        browser: &Browser,
        db: &Db,
        query: &str,
        pause_ms: u64,
    ) -> Result<Vec<Job>> {
        let page_hosts: Vec<_> = browser
            .get_page_urls()
            .await?
            .into_iter()
            .filter_map(|u| host_of(&u))
            .collect();
        if !page_hosts
            .iter()
            .any(|h| h.contains("efinancialcareers.com"))
        {
            bail!("eFinancialCareers requires open efinancialcareers.com tab in Brave");
        }

        let search_url = self.build_search_url(query);
        let page = browser.new_tab(&search_url).await?;

        if !Self::wait_for_jobs(&page).await? {
            page.close().await.ok();
            bail!("eFinancialCareers search page did not load job cards");
        }

        let total_jobs = Self::scrape_total(&page).await?;
        if total_jobs == 0 {
            page.close().await.ok();
            return Ok(Vec::new());
        }

        let mut all_jobs = Vec::new();
        let mut processed_ids: HashSet<String> = HashSet::new();
        let mut checked_count = 0usize;
        let _guard = CursorGuard::new();

        let mut no_progress = 0usize;
        loop {
            let cards = Self::scrape_page(&page).await?;
            let new_cards: Vec<_> = cards
                .into_iter()
                .filter(|c| processed_ids.insert(c.external_id.clone()))
                .collect();

            if new_cards.is_empty() {
                no_progress += 1;
                if no_progress >= 2 {
                    break;
                }
            } else {
                no_progress = 0;
            }

            for card in &new_cards {
                checked_count += 1;
                if db
                    .find_job_id(&Platform::Efinancialcareers, &card.external_id)
                    .await?
                    .is_some()
                {
                    eprint!(
                        "\r    Progress: {:>5}/{:<5} {:.40}\x1B[K",
                        checked_count, total_jobs, ""
                    );
                    continue;
                }

                sleep(Duration::from_millis(pause_ms)).await;

                let detail = match self.fetch_detail(browser, &card.url).await {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!(
                            "    Warning: failed to fetch detail for {}: {}",
                            card.external_id, e
                        );
                        continue;
                    }
                };

                let job = self.build_job(card, detail);
                db.upsert_job(&job).await?;
                all_jobs.push(job);

                eprint!(
                    "\r    Progress: {:>5}/{:<5} {:.40}\x1B[K",
                    checked_count, total_jobs, card.title
                );
            }

            if !Self::click_show_more(&page, pause_ms).await {
                break;
            }
        }

        page.close().await.ok();
        Ok(all_jobs)
    }

    async fn sync_applications(
        &self,
        browser: &Browser,
        db: &Db,
        pause_ms: u64,
        limit: Option<usize>,
    ) -> Result<usize> {
        let page_hosts: Vec<_> = browser
            .get_page_urls()
            .await?
            .into_iter()
            .filter_map(|u| host_of(&u))
            .collect();
        if !page_hosts
            .iter()
            .any(|h| h.contains("efinancialcareers.com"))
        {
            bail!("eFinancialCareers requires open efinancialcareers.com tab in Brave");
        }

        let page = browser
            .new_tab("https://www.efinancialcareers.com/myefc/my-jobs")
            .await?;
        let _ = wait_for_element(&page, &["efc-my-jobs"], None, None).await;
        sleep(Duration::from_millis(pause_ms)).await;

        let auth: AuthResult = page.evaluate(EXTRACT_AUTH_JS).await?.into_value()?;
        let auth = match auth {
            AuthResult::Ok(info) => info,
            AuthResult::Error { error } => bail!("{}", error),
        };
        if auth.token.is_empty() || auth.jobseeker_id.is_empty() {
            bail!("missing efinancialcareers auth token or jobseeker id");
        }

        let fetch_js = FETCH_APPLICATIONS_JS
            .replace("__TOKEN__", &serde_json::to_string(&auth.token)?)
            .replace(
                "__JOBSEEKER_ID__",
                &serde_json::to_string(&auth.jobseeker_id)?,
            );
        let result: ApplicationsResult = page.evaluate(fetch_js.as_str()).await?.into_value()?;
        page.close().await.ok();

        let raw_items = match result {
            ApplicationsResult::Ok { applied } => applied,
            ApplicationsResult::Error { error } => bail!("{}", error),
        };

        let items: Vec<ApplicationItem> = raw_items
            .into_iter()
            .filter_map(|raw| match raw.try_into() {
                Ok(item) => Some(item),
                Err(e) => {
                    eprintln!("  Warning: skipping malformed application item: {}", e);
                    None
                }
            })
            .take(limit.unwrap_or(usize::MAX))
            .collect();

        let http = reqwest::Client::new();
        let mut missing: Vec<&ApplicationItem> = Vec::new();
        for item in &items {
            if db
                .find_job_id(&Platform::Efinancialcareers, &item.external_id)
                .await?
                .is_none()
            {
                missing.push(item);
            }
        }

        let mut descriptions: HashMap<String, String> = HashMap::new();
        if !missing.is_empty() {
            for chunk in missing.chunks(BATCH_CHUNK_SIZE) {
                let ids: Vec<&str> = chunk
                    .iter()
                    .map(|i| i.internal_job_id.as_str())
                    .filter(|id| !id.is_empty())
                    .collect();
                if ids.is_empty() {
                    continue;
                }
                let url = format!(
                    "https://job.efinancialcareers.com/api/v1/jobs/batch?job_ids={}&response_properties=title,summary,description",
                    ids.join(",")
                );
                let res = http
                    .get(&url)
                    .header("Authorization", format!("Bearer {}", auth.token))
                    .header("Accept", "application/json")
                    .send()
                    .await?;
                if !res.status().is_success() {
                    let body = res.text().await.unwrap_or_default();
                    bail!("batch job fetch failed: {}", body);
                }
                let batch: BatchResponse = res.json().await?;
                for job in batch.data {
                    if let Some(text) = Self::html_to_text(&job.description) {
                        descriptions.insert(job.id, text);
                    }
                }
            }
        }

        let mut synced = 0usize;
        let _guard = CursorGuard::new();

        for (processed, item) in items.iter().enumerate() {
            let (job_id, is_new) = if let Some(id) = db
                .find_job_id(&Platform::Efinancialcareers, &item.external_id)
                .await?
            {
                (id, false)
            } else {
                let description = descriptions
                    .get(&item.internal_job_id)
                    .cloned()
                    .unwrap_or_default();
                let salary = item.salary.clone();
                let budget = Budget::parse(&salary, Some("year"))
                    .map(|b| b.to_string())
                    .or_else(|| Some(salary.clone()).filter(|b| !b.is_empty()));

                let job = Job {
                    id: None,
                    platform: Platform::Efinancialcareers,
                    external_id: item.external_id.clone(),
                    title: item.title.clone(),
                    description: Some(description.clone()).filter(|d| !d.is_empty()),
                    url: item.url.clone(),
                    budget,
                    tags: Vec::new(),
                    raw: Data::Efinancialcareers {
                        detail: EfinancialcareersJobDetail {
                            company: item.company.clone(),
                            location: item.location.clone(),
                            employment_type: item.employment_type.clone(),
                            salary: salary.clone(),
                            description,
                            ..Default::default()
                        },
                    },
                    created_at: item.applied_at,
                    updated_at: Utc::now(),
                    liked: None,
                    note: None,
                    applied_at: None,
                };

                (db.upsert_job(&job).await?, true)
            };

            let stored_applied = db
                .get_job(job_id)
                .await?
                .and_then(|j| j.applied_at)
                .is_some();
            if !stored_applied {
                db.set_applied(job_id, None, item.applied_at).await?;
            }

            if is_new || !stored_applied {
                synced += 1;
            }

            let label = if item.title.is_empty() {
                item.external_id.as_str()
            } else {
                item.title.as_str()
            };
            eprint!(
                "\r  Progress {}/{}: {:.40}",
                processed + 1,
                items.len(),
                label
            );
        }
        eprintln!();

        Ok(synced)
    }
}

impl Default for EfinancialcareersScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct EfinancialcareersConfig {
    pub work_arrangement: String,
    pub min_salary: u32,
    pub currency_code: String,
    pub language: String,
}

impl Default for EfinancialcareersConfig {
    fn default() -> Self {
        Self {
            work_arrangement: "REMOTE".to_string(),
            min_salary: 100_000,
            currency_code: "USD".to_string(),
            language: "en".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_search_url_with_keyword() {
        let scraper = EfinancialcareersScraper::new();
        let url = scraper.build_search_url("developer");
        assert!(url.starts_with("https://www.efinancialcareers.com/jobs/remote/developer?"));
        assert!(url.contains("filters.workArrangementType=REMOTE"));
        assert!(url.contains("filters.minSalary=100000"));
        assert!(url.contains("q=developer"));
    }

    #[test]
    fn test_build_search_url_multi_word_keyword() {
        let scraper = EfinancialcareersScraper::new();
        let url = scraper.build_search_url("Rust Developer");
        // Path uses first word only; multi-word query lives in q= param.
        assert!(url.starts_with("https://www.efinancialcareers.com/jobs/remote/Rust?"));
        assert!(url.contains("q=Rust+Developer"));
    }

    #[test]
    fn test_build_search_url_empty_keyword() {
        let scraper = EfinancialcareersScraper::new();
        let url = scraper.build_search_url("");
        assert!(url.starts_with("https://www.efinancialcareers.com/jobs/remote?"));
        assert!(url.contains("q="));
    }
}
