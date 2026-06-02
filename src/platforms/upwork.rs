use crate::browser::BrowserExt;
use crate::models::{Job, JobStatus, Platform, Reaction};
use crate::platforms::PlatformClient;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chromiumoxide::browser::Browser;
use clap::ValueEnum;
use serde_json::Value;

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
    All, // no param (default on Upwork = all tiers)
    Expert,       // contractor_tier=3
    Intermediate, // contractor_tier=2
    BothUpper,    // contractor_tier=2,3
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
        let tier_param = match self.tier.unwrap_or(UpworkTier::All) {
            UpworkTier::All => None,
            UpworkTier::Expert => Some("contractor_tier=3"),
            UpworkTier::Intermediate => Some("contractor_tier=2"),
            UpworkTier::BothUpper => Some("contractor_tier=2,3"),
        };

        let rate_param = self.hourly_rate_min.map(|r| format!("hourly_rate={}-", r));

        let mut parts = vec![
            format!("q={}", urlencoding::encode(&self.query)),
            "sort=relevance%2Bdesc".to_string(),
            "t=0".to_string(),
            "client_hires=1-9,10-".to_string(),
        ];

        if let Some(t) = tier_param {
            parts.push(t.to_string());
        }

        if let Some(r) = rate_param {
            parts.push(r);
        }

        format!(
            "https://www.upwork.com/nx/search/jobs/?{}",
            parts.join(" &")
        )
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

    /// Visit individual job page and scrape full details
    pub async fn fetch_job_detail(&self, browser: &Browser, job_url: &str) -> Result<JobDetail> {
        let page = browser.new_tab(job_url).await?;

        // Wait for job details to load
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
            anyhow::bail!("Job detail page did not load");
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let detail: JobDetail = page
            .evaluate(
                r#"
                (() => {
                    const text = document.body.innerText;
                    
                    // Proposals - look for "20 to 50" pattern near "Proposals"
                    const proposalsMatch = text.match(/Proposals[:\s]*(?:Close[^\d]*)?(\d+\s+to\s+\d+|\d+)/i);
                    const proposals = proposalsMatch ? proposalsMatch[1].trim() : '';
                    
                    // Last viewed by client
                    const viewedMatch = text.match(/Last viewed by client[:\s]*([^\n]+)/i);
                    const last_viewed = viewedMatch ? viewedMatch[1].trim().replace(/Close the tooltip.*$/, '').trim() : '';
                    
                    // Interviewing
                    const interviewingMatch = text.match(/Interviewing[:\s]*(\d+)/i);
                    const interviewing = interviewingMatch ? interviewingMatch[1] : '';
                    
                    // Invites sent
                    const invitesMatch = text.match(/Invites sent[:\s]*(\d+)/i);
                    const invites_sent = invitesMatch ? invitesMatch[1] : '';
                    
                    // Unanswered invites
                    const unansweredMatch = text.match(/Unanswered invites[:\s]*(\d+)/i);
                    const unanswered_invites = unansweredMatch ? unansweredMatch[1] : '';
                    
                    // Full description - find the largest text block that's not nav/footer
                    const sections = Array.from(document.querySelectorAll('section, [data-test="Description"], [data-test="job-description"]'));
                    let description = '';
                    for (const section of sections) {
                        const t = section.innerText?.trim() || '';
                        if (t.length > description.length && t.length > 200 && !t.includes('Footer navigation')) {
                            description = t;
                        }
                    }
                    
                    // Exact budget from structured data if available
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
                    
                    // Fallback: look for budget in text
                    if (!exact_budget) {
                        const budgetMatch = text.match(/\$\d+[\d,]*\.?\d*\s*[-]\s*\$\d+[\d,]*\.?\d*/)
                            || text.match(/Budget[:\s]*([^\n]{0,50})/i);
                        exact_budget = budgetMatch ? budgetMatch[0].replace(/\s+/g, ' ').trim() : '';
                    }
                    
                    return {
                        proposals,
                        last_viewed,
                        interviewing,
                        invites_sent,
                        unanswered_invites,
                        description,
                        exact_budget
                    };
                })()
                "#,
            )
            .await?
            .into_value()?;

        page.close().await.ok();
        Ok(detail)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JobDetail {
    pub proposals: String,
    pub last_viewed: String,
    pub interviewing: String,
    pub invites_sent: String,
    pub unanswered_invites: String,
    pub description: String,
    pub exact_budget: String,
}

#[async_trait]
impl PlatformClient for UpworkScraper {
    fn name(&self) -> &'static str {
        "upwork"
    }

    async fn fetch_with_browser(&self, browser: &Browser, query: &str) -> Result<Vec<Job>> {
        let search_url = Self::build_search_url(query, self.tier, self.hourly_rate_min);

        let hosts = browser.get_page_hosts().await?;
        let has_upwork_tab = hosts.iter().any(|h| h.contains("upwork.com"));

        if !has_upwork_tab {
            anyhow::bail!("Upwork requires open upwork.com tab in Brave");
        }

        let page = browser.new_tab(&search_url).await?;

        // Wait for job cards or CAPTCHA
        let mut found = false;
        for i in 0..120 {
            let is_challenge = page
                .evaluate(
                    r#"
                document.title.includes('Just a moment') || 
                document.title.includes('Challenge') ||
                !!document.querySelector('#cf-challenge-running')
            "#,
                )
                .await?
                .into_value::<bool>()?;

            if !is_challenge
                && page
                    .find_element("article[data-test='JobTile']")
                    .await
                    .is_ok()
            {
                found = true;
                break;
            }

            if i == 30 {
                eprintln!("  Upwork showing CAPTCHA. Login in Brave first, then retry.");
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        if !found {
            anyhow::bail!("Upwork job cards did not appear. Login at upwork.com in Brave first.");
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let raw_jobs: Vec<serde_json::Value> = page
            .evaluate(
                r#"
                Array.from(document.querySelectorAll("article[data-test='JobTile']")).map(el => {
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
                })
                "#,
            )
            .await?
            .into_value()?;

        let jobs: Vec<Job> = raw_jobs
            .iter()
            .filter_map(|v| raw_to_job(v).ok())
            .filter(|j| !j.external_id.is_empty())
            .collect();

        page.close().await.ok();
        Ok(jobs)
    }

    async fn react(&self, _job: &Job, _action: Reaction) -> Result<()> {
        Err(anyhow!("Upwork react not yet implemented"))
    }
}

fn raw_to_job(v: &Value) -> Result<Job> {
    let external_id = v["external_id"]
        .as_str()
        .ok_or_else(|| anyhow!("missing external_id"))?
        .to_string();

    let posted_at = v["posted_at_text"].as_str().and_then(parse_upwork_time);

    Ok(Job {
        id: None,
        platform: Platform::Upwork,
        external_id,
        title: v["title"].as_str().unwrap_or("").to_string(),
        description: v["description"].as_str().map(|s| s.to_string()),
        url: v["url"].as_str().unwrap_or("").to_string(),
        posted_at,
        budget: v["budget"].as_str().map(|s| s.to_string()),
        tags: v["tags"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        raw: v.clone(),
        status: JobStatus::New,
        created_at: None,
        updated_at: None,
    })
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
        assert!(url.contains("q=quant%20trading"));
        assert!(!url.contains("hourly_rate")); // no min rate by default
        assert!(!url.contains("contractor_tier")); // All by default
    }

    #[test]
    fn test_build_search_url_with_tier_and_rate() {
        let url =
            UpworkScraper::build_search_url("quant trading", Some(UpworkTier::BothUpper), Some(65));
        assert!(url.contains("q=quant%20trading"));
        assert!(url.contains("hourly_rate=65-"));
        assert!(url.contains("contractor_tier=2,3"));
    }

    #[test]
    fn test_upwork_search_params_defaults() {
        let params = UpworkSearchParams::new("rust");
        let url = params.build_url();
        assert!(url.contains("q=rust"));
        assert!(!url.contains("contractor_tier")); // Expert default, no param
    }

    #[test]
    fn test_upwork_search_params_builder() {
        let url = UpworkSearchParams::new("rust")
            .tier(Some(UpworkTier::BothUpper))
            .hourly_rate_min(Some(65))
            .build_url();
        assert!(url.contains("contractor_tier=2,3"));
        assert!(url.contains("hourly_rate=65-"));
    }
}
