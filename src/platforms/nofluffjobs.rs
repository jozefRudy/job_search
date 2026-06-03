use crate::browser::BrowserExt;
use crate::db::Db;
use crate::models::{Data, Job, JobStatus, NoFluffJobCard, NoFluffJobDetail, Platform, Reaction};
use crate::platforms::PlatformClient;
use anyhow::{Result, bail};
use async_trait::async_trait;
use chromiumoxide::browser::Browser;

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

        let search_url = self.build_search_url(query);
        let page = browser.new_tab(&search_url).await?;

        if !Self::wait_for_jobs(&page).await? {
            bail!("NoFluffJobs job cards did not appear within 30s");
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let mut all_jobs: Vec<Job> = Vec::new();
        let platform = Platform::NoFluffJobs;

        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(pause_ms)).await;

            let raw_jobs = Self::scrape_page(&page).await?;
            let mut stopped = false;

            for v in &raw_jobs {
                if db.job_exists(&platform, &v.external_id).await? {
                    eprintln!(
                        "  Stopping: '{}' already in DB ({})",
                        v.title, v.external_id
                    );
                    stopped = true;
                    break;
                }

                let job_url = v.url.clone();

                match self.fetch_job_detail(browser, &job_url).await {
                    Ok(detail) => {
                        let posted_at = v
                            .posted_at_text
                            .as_ref()
                            .and_then(|t| parse_nofluff_time(t));
                        let job = Job {
                            id: None,
                            platform,
                            external_id: v.external_id.clone(),
                            title: v.title.clone(),
                            description: None,
                            url: v.url.clone(),
                            posted_at,
                            budget: v.budget.clone(),
                            tags: v.tags.clone(),
                            raw: Data::Nofluffjobs { detail },
                            status: JobStatus::New,
                            created_at: None,
                            updated_at: None,
                        };
                        db.upsert_job(&job).await?;
                        all_jobs.push(job);
                    }
                    Err(e) => {
                        eprintln!("    Warning: failed to fetch detail for {}: {}", v.title, e);
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(pause_ms)).await;
            }

            if stopped {
                break;
            }

            eprintln!(
                "  Page: +{} jobs (total: {})",
                raw_jobs
                    .iter()
                    .filter(|v| !v.external_id.is_empty())
                    .count(),
                all_jobs.len()
            );

            if !Self::click_load_more(&page, pause_ms).await {
                break;
            }
        }

        page.close().await.ok();
        Ok(all_jobs)
    }

    async fn react(&self, _job: &Job, _action: Reaction) -> Result<()> {
        bail!("NoFluffJobs react not yet implemented")
    }
}

impl NoFluffJobsScraper {
    /// Wait for job cards to appear. Returns true if found.
    pub async fn wait_for_jobs(page: &chromiumoxide::Page) -> Result<bool> {
        for _ in 0..60 {
            if page.find_element("a.posting-list-item").await.is_ok() {
                return Ok(true);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        Ok(false)
    }

    /// Scrape job cards from current page.
    pub async fn scrape_page(page: &chromiumoxide::Page) -> Result<Vec<NoFluffJobCard>> {
        let cards: Vec<NoFluffJobCard> = page
            .evaluate(
                r#"
                Array.from(document.querySelectorAll("a.posting-list-item")).map(el => {
                    const titleEl = el.querySelector('h3');
                    const tagsEls = el.querySelectorAll('.posting-tag');
                    const salaryEl = el.querySelector('[class*="salary"]');
                    const timeEl = el.querySelector('time');
                    const url = el.href;
                    const idMatch = url?.match(/\/job\/([^\/?#]+)/);
                    const id = idMatch ? idMatch[1] : '';
                    return {
                        external_id: id,
                        title: titleEl?.textContent?.trim() || "",
                        url: url || "",
                        budget: salaryEl?.textContent?.trim() || null,
                        posted_at_text: timeEl?.textContent?.trim() || null,
                        tags: Array.from(tagsEls).map(s => s.textContent.trim()).filter(Boolean)
                    };
                })
                "#,
            )
            .await?
            .into_value()?;
        Ok(cards)
    }

    /// Click "Load More" button and wait for new jobs. Returns true if more jobs loaded.
    pub async fn click_load_more(page: &chromiumoxide::Page, pause_ms: u64) -> bool {
        let has_more: Option<String> = page
            .evaluate(
                r#"
                (() => {
                    const btn = Array.from(document.querySelectorAll('button, [role="button"], a'))
                        .find(el => /load\s*more|show\s*more|view\s*more|see\s*more/i.test(el.textContent || ''));
                    if (btn && !btn.disabled && btn.offsetParent !== null) {
                        btn.scrollIntoView({ block: 'center' });
                        return 'click';
                    }
                    return 'none';
                })()
                "#,
            )
            .await
            .ok()
            .and_then(|v| v.into_value().ok());

        if has_more.as_deref() != Some("click") {
            return false;
        }

        let prev_count: i32 = page
            .evaluate(r#"document.querySelectorAll("a.posting-list-item").length"#)
            .await
            .ok()
            .and_then(|v| v.into_value().ok())
            .unwrap_or(0);

        let _ = page
            .evaluate(
                r#"
                (() => {
                    const btn = Array.from(document.querySelectorAll('button, [role="button"], a'))
                        .find(el => /load\s*more|show\s*more|view\s*more|see\s*more/i.test(el.textContent || ''));
                    if (btn) btn.click();
                })()
                "#,
            )
            .await;

        tokio::time::sleep(tokio::time::Duration::from_millis(pause_ms)).await;

        for _ in 0..3 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            let count: i32 = page
                .evaluate(r#"document.querySelectorAll("a.posting-list-item").length"#)
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

    pub async fn fetch_job_detail(
        &self,
        browser: &Browser,
        job_url: &str,
    ) -> Result<NoFluffJobDetail> {
        let page = browser.new_tab(job_url).await?;

        let mut found = false;
        for _ in 0..3 {
            if page.find_element("h1").await.is_ok()
                || page.find_element("[data-test='job-title']").await.is_ok()
            {
                found = true;
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        if !found {
            page.close().await.ok();
            bail!("NoFluffJobs detail page did not load");
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let detail: NoFluffJobDetail = page
            .evaluate(
                r#"
                (() => {
                    const text = document.body.innerText;

                    let company = '';
                    const h1 = document.querySelector('h1');
                    if (h1) {
                        const next = h1.parentElement?.querySelector('a, h2, div');
                        if (next) company = next.textContent.trim();
                    }
                    if (!company) {
                        const lines = text.split('\n').map(s => s.trim()).filter(Boolean);
                        if (lines.length > 1) company = lines[1];
                    }

                    let seniority = '';
                    const seniorityMatch = text.match(/\b(Senior|Mid|Junior|Expert|Lead|Principal)\b/i);
                    if (seniorityMatch) seniority = seniorityMatch[1];

                    let remote = '';
                    if (text.includes('Fully remote')) remote = 'Fully remote';
                    else if (text.includes('Remote')) remote = 'Remote';
                    else if (text.includes('Hybrid')) remote = 'Hybrid';
                    else if (text.includes('On-site')) remote = 'On-site';

                    let locations = [];
                    const locMatch = text.match(/Locations?:[\s\S]*?(?=\n\s*\n|Show on map|Offer valid|$)/i);
                    if (locMatch) {
                        const locText = locMatch[0].replace(/Locations?:/, '').replace(/Show on map/, '');
                        locations = locText.split(/,|\n/)
                            .map(s => s.trim())
                            .filter(s => s && s.length > 2 && s !== 'Remote');
                    }

                    let must_have = [];
                    const mustMatch = text.match(/Must have[\s\S]*?(?=Requirements description|Nice to have|Offer description|$)/i);
                    if (mustMatch) {
                        const lines = mustMatch[0].replace('Must have', '').trim().split('\n');
                        for (const line of lines) {
                            const trimmed = line.trim();
                            if (trimmed && trimmed.length > 1 && trimmed.length < 50 && !trimmed.includes(':')) {
                                must_have.push(trimmed);
                            }
                        }
                    }

                    let requirements = '';
                    const reqMatch = text.match(/Requirements description[\s\S]*?(?=Offer description|Job details|$)/i);
                    if (reqMatch) requirements = reqMatch[0].replace('Requirements description', '').trim();

                    let offer_description = '';
                    const offerMatch = text.match(/Offer description[\s\S]*?(?=Job details|$)/i);
                    if (offerMatch) offer_description = offerMatch[0].replace('Offer description', '').trim();

                    let offer_valid_until = '';
                    const validMatch = text.match(/Offer valid until[:\s]*([^\n(]+)/i);
                    if (validMatch) offer_valid_until = validMatch[1].trim();

                    return { company, seniority, remote, locations, must_have, requirements, offer_description, offer_valid_until };
                })()
                "#,
            )
            .await?
            .into_value()?;

        page.close().await.ok();
        Ok(detail)
    }
}

fn parse_nofluff_time(text: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let now = chrono::Utc::now();
    let text = text.to_lowercase();
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    let n: i64 = parts[0].parse().ok()?;
    match parts[1] {
        "minute" | "minutes" => Some(now - chrono::Duration::minutes(n)),
        "hour" | "hours" => Some(now - chrono::Duration::hours(n)),
        "day" | "days" => Some(now - chrono::Duration::days(n)),
        "week" | "weeks" => Some(now - chrono::Duration::days(n * 7)),
        "month" | "months" => Some(now - chrono::Duration::days(n * 30)),
        "yesterday" => Some(now - chrono::Duration::days(1)),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub struct NoFluffJobsConfig {
    pub path: String,
    pub min_salary_eur: Option<u32>,
    pub employment: Option<String>,
    pub language: Option<String>,
}

impl Default for NoFluffJobsConfig {
    fn default() -> Self {
        Self {
            path: "remote".to_string(),
            min_salary_eur: None,
            employment: None,
            language: None,
        }
    }
}

pub struct NoFluffJobsScraper {
    config: NoFluffJobsConfig,
}

impl Default for NoFluffJobsScraper {
    fn default() -> Self {
        Self::new()
    }
}

impl NoFluffJobsScraper {
    pub fn new() -> Self {
        Self {
            config: NoFluffJobsConfig::default(),
        }
    }

    pub fn with_config(config: NoFluffJobsConfig) -> Self {
        Self { config }
    }

    pub fn build_search_url(&self, query: &str) -> String {
        let mut criteria: Vec<String> = Vec::new();
        if let Some(emp) = &self.config.employment {
            criteria.push(format!("employment={}", emp));
        }
        if let Some(salary) = self.config.min_salary_eur {
            criteria.push(format!("salary>eur{}m", salary));
        }
        if let Some(lang) = &self.config.language {
            criteria.push(format!("jobLanguage={}", lang));
        }
        if !query.is_empty() {
            criteria.push(format!("keyword={}", query));
        }
        let criteria_str = criteria.join(" ");
        format!(
            "https://nofluffjobs.com/{}?criteria={}",
            self.config.path,
            urlencoding::encode(&criteria_str)
        )
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
        };
        let scraper = NoFluffJobsScraper::with_config(config);
        let url = scraper.build_search_url("senior");
        assert_eq!(
            url,
            "https://nofluffjobs.com/pl/jobs?criteria=keyword%3Dsenior"
        );
    }
}
