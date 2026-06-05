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

    pub fn build_url(&self) -> String {
        // Most-recent feed has reliable "Load More" pagination.
        // Search page uses infinite scroll / fixed results with no load-more button.
        "https://www.upwork.com/nx/find-work/most-recent".to_string()
    }
}

#[derive(Debug, Clone, Default)]
pub struct UpworkScraper {
    pub tier: Option<UpworkTier>,
    pub hourly_rate_min: Option<u32>,
}

impl UpworkScraper {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(tier: Option<UpworkTier>, hourly_rate_min: Option<u32>) -> Self {
        Self {
            tier,
            hourly_rate_min,
        }
    }

    pub fn build_search_url(
        query: &str,
        tier: Option<UpworkTier>,
        hourly_rate_min: Option<u32>,
    ) -> String {
        UpworkSearchParams::new(query)
            .tier(tier)
            .hourly_rate_min(hourly_rate_min)
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
            .evaluate(
                r#"
                (() => {
                    const text = document.body.innerText;

                    // Regex-based fields (no reliable DOM selectors)
                    const proposalsMatch = text.match(/Proposals[:\s]*(?:Close[^\d]*)?(\d+\s+to\s+\d+|\d+)/i);
                    const proposals = proposalsMatch ? proposalsMatch[1].trim() : '';
                    const viewedMatch = text.match(/Last viewed by client[:\s]*([^\n]+)/i);
                    const last_viewed = viewedMatch ? viewedMatch[1].trim().replace(/Close the tooltip.*$/, '').trim() : '';
                    const interviewingMatch = text.match(/Interviewing[:\s]*(\d+)/i);
                    const interviewing = interviewingMatch ? interviewingMatch[1] : '';
                    const invitesMatch = text.match(/Invites sent[:\s]*(\d+)/i);
                    const invites_sent = invitesMatch ? invitesMatch[1] : '';
                    const unansweredMatch = text.match(/Unanswered invites[:\s]*(\d+)/i);
                    const unanswered_invites = unansweredMatch ? unansweredMatch[1] : '';
                    const hiresMatch = text.match(/Hires[:\s]*(\d+)/i);
                    const hires = hiresMatch ? hiresMatch[1] : '';
                    const typeMatch = text.match(/Project type[:\s]*([^\n]+)/i);
                    const project_type = typeMatch ? typeMatch[1].trim() : '';

                    // DOM-based fields (data-cy attributes on <li> elements)
                    const expLi = document.querySelector('[data-cy="expertise"]')?.closest('li');
                    let experience_level = '';
                    if (expLi) {
                        const t = expLi.innerText.replace(/\s+/g, ' ').trim();
                        const m = t.match(/(Entry Level|Intermediate|Expert)/);
                        experience_level = m ? m[1] : '';
                    }

                    const durLi = document.querySelector('[data-cy^="duration"]')?.closest('li');
                    let duration = '';
                    if (durLi) {
                        duration = durLi.innerText.replace(/\s+/g, ' ').trim().replace(/\s*Duration\s*$/, '').trim();
                    }

                    const hoursLi = document.querySelector('[data-cy="clock-hourly"]')?.closest('li');
                    let hours_per_week = '';
                    if (hoursLi) {
                        hours_per_week = hoursLi.innerText.replace(/\s+/g, ' ').trim().replace(/\s*Hourly\s*$/, '').trim();
                    }

                    // Description: prefer specific data-test attribute
                    let description = '';
                    const descEl = document.querySelector('[data-test="Description"]')
                        || document.querySelector('[data-test="job-description"]');
                    if (descEl) {
                        description = descEl.innerText?.trim() || '';
                    }
                    // Fallback to longest non-history section
                    if (!description || description.length < 200) {
                        const sections = Array.from(document.querySelectorAll('section'));
                        for (const section of sections) {
                            const t = section.innerText?.trim() || '';
                            if (t.length > description.length
                                && t.length > 200
                                && !t.includes('Footer navigation')
                                && !t.includes('Rating is')
                                && !t.includes('To freelancer:')
                                && !t.includes('Billed: $')) {
                                description = t;
                            }
                        }
                    }

                    // Budget: NUXT → DOM selector → regex fallback
                    let exact_budget = '';
                    try {
                        if (window.__NUXT__?.state?.job?.budget) {
                            const b = window.__NUXT__.state.job.budget;
                            if (b.hourlyBudgetMin && b.hourlyBudgetMax) {
                                exact_budget = `$${b.hourlyBudgetMin} - $${b.hourlyBudgetMax}/hr`;
                            } else if (b.amount) {
                                exact_budget = `$${b.amount}`;
                            }
                        }
                    } catch(e) {}
                    if (!exact_budget) {
                        const budgetLi = document.querySelector('[data-cy="clock-timelog"]')?.closest('li');
                        if (budgetLi) {
                            exact_budget = budgetLi.innerText.replace(/\s+/g, ' ').trim().replace(/\s*Hourly\s*$/, '').trim();
                        }
                    }
                    if (!exact_budget) {
                        const budgetMatch = text.match(/\$\d+[\d,]*\.?\d*\s*[-]\s*\$\d+[\d,]*\.?\d*/)
                            || text.match(/Budget[:\s]*([^\n]{0,50})/i);
                        exact_budget = budgetMatch ? budgetMatch[0].replace(/\s+/g, ' ').trim() : '';
                    }

                    return { proposals, last_viewed, interviewing, invites_sent, unanswered_invites, description, exact_budget, experience_level, hires, project_type, duration, hours_per_week };
                })()
                "#,
            )
            .await?
            .into_value()?;

        page.close().await.ok();
        Ok(detail)
    }

    /// Scrape job cards from current page (search page or most-recent feed).
    pub async fn scrape_page(page: &chromiumoxide::Page) -> Result<Vec<UpworkJobCard>> {
        let cards: Vec<UpworkJobCard> = page
            .evaluate(
                r#"
                (() => {
                    // Try search-page selector first
                    const searchTiles = Array.from(document.querySelectorAll("article[data-test='JobTile']"));
                    if (searchTiles.length > 0) {
                        return searchTiles.map(el => {
                            const titleLink = el.querySelector('a');
                            const budgetEl = el.querySelector("[data-test='job-type-label']");
                            const timeEl = el.querySelector('small');
                            const skillsEls = el.querySelectorAll("[data-test='token']");
                            const uid = el.getAttribute("data-ev-job-uid");
                            return {
                                external_id: uid || "",
                                title: titleLink?.textContent?.trim() || "",
                                description: null,
                                url: titleLink?.href ? new URL(titleLink.href, location.href).href : "",
                                budget: budgetEl?.textContent?.trim() || null,
                                posted_at_text: timeEl?.textContent?.trim() || null,
                                tags: Array.from(skillsEls).map(s => s.textContent.trim()).filter(Boolean)
                            };
                        });
                    }

                    // Fallback: most-recent feed page
                    const sections = Array.from(document.querySelectorAll('section.air3-card-section')).filter(s => {
                        return s.querySelector('a') && !s.querySelector('.air3-skeleton-shape');
                    });
                    return sections.map(s => {
                        const titleLink = s.querySelector('.job-tile-title a') || s.querySelector('a.air3-link');
                        const allSpans = Array.from(s.querySelectorAll('span, strong'));

                        let posted_at_text = '';
                        for (const el of allSpans) {
                            const t = el.textContent.trim();
                            if (/\d+\s+(minute|hour|day|week)s?\s+ago/i.test(t) || /just\s+now/i.test(t)) {
                                posted_at_text = t;
                                break;
                            }
                        }

                        let budget = '';
                        const fixedPrice = allSpans.find(el => el.textContent.trim() === 'Fixed-price');
                        const hourly = allSpans.find(el => el.textContent.trim() === 'Hourly');
                        if (fixedPrice) {
                            const budgetSpan = allSpans.find(el => el.textContent.includes('Est. Budget:'));
                            if (budgetSpan) {
                                const m = budgetSpan.textContent.match(/\$[\d,]+/);
                                budget = m ? 'Fixed-price: ' + m[0] : 'Fixed-price';
                            } else {
                                budget = 'Fixed-price';
                            }
                        } else if (hourly) {
                            budget = 'Hourly';
                        }

                        const skills = Array.from(s.querySelectorAll('.air3-token')).map(el => el.textContent.trim()).filter(Boolean);

                        const url = titleLink?.href || '';
                        const idMatch = url.match(/~(\d+)/);
                        const external_id = idMatch ? idMatch[1] : '';

                        return {
                            external_id,
                            title: titleLink?.textContent?.trim() || '',
                            description: null,
                            url: url ? new URL(url, location.href).href : '',
                            budget: budget || null,
                            posted_at_text: posted_at_text || null,
                            tags: skills
                        };
                    });
                })()
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
                    const btn = document.querySelector('[data-test="Load More Jobs"]')
                        || document.querySelector('button[class*="load"]')
                        || document.querySelector('button[class*="Load"]')
                        || Array.from(document.querySelectorAll('button, [role="button"]'))
                            .find(el => /load\s*more|show\s*more|view\s*more/i.test(el.textContent || ''));
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

        let _ = page
            .evaluate(
                r#"
                (() => {
                    const btn = document.querySelector('[data-test="Load More Jobs"]')
                        || document.querySelector('button[class*="load"]')
                        || document.querySelector('button[class*="Load"]')
                        || Array.from(document.querySelectorAll('button, [role="button"]'))
                            .find(el => /load\s*more|show\s*more|view\s*more/i.test(el.textContent || ''));
                    if (btn) btn.click();
                })()
                "#,
            )
            .await;

        // Count real job sections (most-recent page uses section.air3-card-section)
        let prev_count: i32 = page
            .evaluate(r#"
                Array.from(document.querySelectorAll('section.air3-card-section')).filter(s => s.querySelector('a') && !s.querySelector('.air3-skeleton-shape')).length
            "#)
            .await
            .ok()
            .and_then(|v| v.into_value().ok())
            .unwrap_or(0);

        tokio::time::sleep(tokio::time::Duration::from_millis(pause_ms)).await;

        for _ in 0..30 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            let count: i32 = page
                .evaluate(r#"
                    Array.from(document.querySelectorAll('section.air3-card-section')).filter(s => s.querySelector('a') && !s.querySelector('.air3-skeleton-shape')).length
                "#)
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

    /// Wait for job cards to appear (or CAPTCHA). Returns true if cards found.
    pub async fn wait_for_jobs(page: &chromiumoxide::Page) -> Result<bool> {
        for i in 0..120 {
            let is_challenge: bool = page
                .evaluate(
                    r#"
                    document.title.includes('Just a moment') ||
                    document.title.includes('Challenge') ||
                    !!document.querySelector('#cf-challenge-running')
                "#,
                )
                .await?
                .into_value()?;

            let has_cards: bool = page
                .evaluate(
                    r#"
                    !!document.querySelector("article[data-test='JobTile']") ||
                    Array.from(document.querySelectorAll('section.air3-card-section')).some(s => s.querySelector('a') && !s.querySelector('.air3-skeleton-shape'))
                "#,
                )
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
        let search_url = Self::build_search_url(query, self.tier, self.hourly_rate_min);

        let hosts = browser.get_page_hosts().await?;
        if !hosts.iter().any(|h| h.contains("upwork.com")) {
            bail!("Upwork requires open upwork.com tab in Brave");
        }

        let page = browser.new_tab(&search_url).await?;

        if !Self::wait_for_jobs(&page).await? {
            bail!("Upwork job cards did not appear. Login at upwork.com in Brave first.");
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let mut all_jobs: Vec<Job> = Vec::new();

        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(pause_ms)).await;

            let raw_jobs = Self::scrape_page(&page).await?;

            for v in &raw_jobs {
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
                            created_at: posted,
                            updated_at: None,
                            note: None,
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
        let url = UpworkScraper::build_search_url("quant trading", None, None);
        assert!(url.contains("most-recent"));
    }

    #[test]
    fn test_build_search_url_ignores_query_and_filters() {
        // most-recent feed does not support query/filters in URL.
        let url =
            UpworkScraper::build_search_url("quant trading", Some(UpworkTier::BothUpper), Some(65));
        assert!(url.contains("most-recent"));
        assert!(!url.contains("q="));
        assert!(!url.contains("hourly_rate"));
        assert!(!url.contains("contractor_tier"));
    }

    #[test]
    fn test_upwork_search_params_defaults() {
        let params = UpworkSearchParams::new("rust");
        let url = params.build_url();
        assert!(url.contains("most-recent"));
    }

    #[test]
    fn test_upwork_search_params_builder_ignores_filters() {
        let url = UpworkSearchParams::new("rust")
            .tier(Some(UpworkTier::BothUpper))
            .hourly_rate_min(Some(65))
            .build_url();
        assert!(url.contains("most-recent"));
        assert!(!url.contains("contractor_tier"));
        assert!(!url.contains("hourly_rate"));
    }
}
