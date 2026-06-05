use crate::browser::BrowserExt;
use crate::db::Db;
use crate::models::{Data, Job, Platform, UpworkJobDetail};
use crate::platforms::PlatformClient;
use anyhow::{Result, anyhow, bail};
use async_trait::async_trait;
use chromiumoxide::browser::Browser;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Job card as scraped from the Upwork list page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpworkJobCard {
    pub external_id: String,
    pub title: String,
    pub description: Option<String>,
    pub url: String,
    pub budget: Option<String>,
    pub posted_at_text: Option<String>,
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

        let mut found = false;
        for _ in 0..30 {
            if page.find_element("[data-test='Description']").await.is_ok()
                || page.find_element("[class*='description']").await.is_ok()
            {
                found = true;
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        if !found {
            page.close().await.ok();
            bail!("Job detail page did not load");
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let detail: UpworkJobDetail = page
            .evaluate(crate::platforms::upwork_js::FETCH_JOB_DETAIL)
            .await?
            .into_value()?;

        page.close().await.ok();
        Ok(detail)
    }

    /// Scrape job cards from current search page.
    pub async fn scrape_page(page: &chromiumoxide::Page) -> Result<Vec<UpworkJobCard>> {
        let cards: Vec<UpworkJobCard> = page
            .evaluate(crate::platforms::upwork_js::SCRAPE_CARDS)
            .await?
            .into_value()?;
        Ok(cards)
    }

    /// Wait for job cards to appear (or CAPTCHA). Returns true if cards found.
    pub async fn wait_for_jobs(page: &chromiumoxide::Page) -> Result<bool> {
        for i in 0..120 {
            let is_challenge: bool = page
                .evaluate(crate::platforms::upwork_js::IS_CHALLENGE)
                .await?
                .into_value()?;

            let has_cards: bool = page
                .evaluate(crate::platforms::upwork_js::HAS_CARDS)
                .await?
                .into_value()?;

            if !is_challenge && has_cards {
                return Ok(true);
            }

            if i == 30 {
                eprintln!("  Upwork showing CAPTCHA. Login in Brave first, then retry.");
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        Ok(false)
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
    ) -> Result<Vec<Job>> {
        let hosts = browser.get_page_hosts().await?;
        if !hosts.iter().any(|h| h.contains("upwork.com")) {
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

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let mut all_jobs: Vec<Job> = Vec::new();
        let mut page_num = 1u32;
        let mut checked_count = 0usize;

        eprint!("\x1B[?25l"); // hide cursor

        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(pause_ms)).await;

            let raw_jobs = Self::scrape_page(&page).await?;

            let mut stop = false;
            for v in &raw_jobs {
                checked_count += 1;
                let is_stale = v
                    .posted_at_text
                    .as_ref()
                    .and_then(|t| parse_upwork_time(t))
                    .is_some_and(|posted| {
                        let age = chrono::Utc::now() - posted;
                        age.num_days() >= 7
                    });

                if is_stale && db.job_exists(&Platform::Upwork, &v.external_id).await? {
                    eprintln!(
                        "  Stopping: '{}' is {} old and already in DB ({})",
                        v.title,
                        v.posted_at_text.as_deref().unwrap_or("?"),
                        v.external_id
                    );
                    stop = true;
                    break;
                }

                let job_url = v.url.clone();

                match self.fetch_job_detail(browser, &job_url).await {
                    Ok(detail) => {
                        let posted = v.posted_at_text.as_ref().and_then(|t| parse_upwork_time(t));
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
                            created_at: posted.unwrap_or_else(chrono::Utc::now),
                            updated_at: chrono::Utc::now(),
                            note: None,
                            applied_at: None,
                        };
                        db.upsert_job(&job).await?;
                        all_jobs.push(job);
                    }
                    Err(e) => {
                        eprintln!("    Warning: failed to fetch detail for {}: {}", v.title, e);
                    }
                }

                eprint!("\r    Progress: {:>5} {:.40}\x1B[K", checked_count, v.title);
                tokio::time::sleep(tokio::time::Duration::from_millis(pause_ms)).await;
            }

            eprintln!(
                "  Page {}: +{} jobs (total: {})",
                page_num,
                raw_jobs
                    .iter()
                    .filter(|v| !v.external_id.is_empty())
                    .count(),
                all_jobs.len()
            );

            if stop {
                break;
            }

            let has_next: bool = page
                .evaluate(crate::platforms::upwork_js::HAS_NEXT_PAGE)
                .await?
                .into_value()?;

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
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }

        eprintln!("\x1B[?25h"); // show cursor
        page.close().await.ok();
        eprintln!("  Total new jobs: {}", all_jobs.len());
        Ok(all_jobs)
    }

    async fn react(&self, _job: &Job, _note: Option<String>) -> Result<()> {
        Err(anyhow!("Upwork react not yet implemented"))
    }
}

fn parse_upwork_time(text: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let now = chrono::Utc::now();
    let text = text.to_lowercase();
    let text = text.strip_prefix("posted ")?;
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    let n: i64 = parts[0].parse().ok()?;
    match parts[1] {
        "minute" | "minutes" | "min" | "mins" => Some(now - chrono::Duration::minutes(n)),
        "hour" | "hours" | "hr" | "hrs" => Some(now - chrono::Duration::hours(n)),
        "day" | "days" => Some(now - chrono::Duration::days(n)),
        "week" | "weeks" => Some(now - chrono::Duration::days(n * 7)),
        "month" | "months" => Some(now - chrono::Duration::days(n * 30)),
        _ => None,
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
}
