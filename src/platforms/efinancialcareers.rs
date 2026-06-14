use crate::browser::{BrowserExt, wait_for_element};
use crate::db::Db;
use crate::models::{Budget, Data, EfinancialcareersJobDetail, Job, Platform, parse_relative_time};
use crate::platforms::PlatformClient;
use crate::term::CursorGuard;
use anyhow::{Result, bail};
use async_trait::async_trait;
use chromiumoxide::browser::Browser;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;
use tokio::time::sleep;

const SCRAPE_CARDS_JS: &str = include_str!("efinancialcareers/scrape_cards.js");
const CLICK_SHOW_MORE_JS: &str = include_str!("efinancialcareers/click_show_more.js");
const SCRAPE_TOTAL_JS: &str = include_str!("efinancialcareers/scrape_total.js");
const EXTRACT_DETAIL_JS: &str = include_str!("efinancialcareers/extract_detail.js");

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
        let hosts = browser.get_page_hosts().await?;
        if !hosts.iter().any(|h| h.contains("efinancialcareers.com")) {
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
                    .job_exists(&Platform::Efinancialcareers, &card.external_id)
                    .await?
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
