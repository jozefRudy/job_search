use super::html;
use crate::browser::{BrowserExt, host_of};
use crate::db::{Db, UpsertResult};
use crate::language::LanguageService;
use crate::models::{Data, EfinancialcareersJobDetail, Job, Platform, Rating, classify_language};
use crate::platforms::{FetchState, PlatformClient};
use crate::term::CursorGuard;
use anyhow::{Result, bail};
use async_trait::async_trait;
use chromiumoxide::browser::Browser;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;
use tokio::time::sleep;

const SCRAPE_CARDS_JS: &str = include_str!("efinancialcareers/scrape_cards.js");
const SEARCH_RESULTS_CONTAINER_JS: &str =
    include_str!("efinancialcareers/search_results_container.js");
const CLICK_SHOW_MORE_JS: &str = include_str!("efinancialcareers/click_show_more.js");
const SCRAPE_TOTAL_JS: &str = include_str!("efinancialcareers/scrape_total.js");

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

#[derive(Debug, Clone, Deserialize)]
struct FacadeJob {
    #[serde(default)]
    description: String,
    #[serde(default)]
    salary: String,
    #[serde(default)]
    position_type: String,
    #[serde(default)]
    employment_type: String,
    #[serde(default)]
    posted_date: String,
    #[serde(default)]
    work_arrangement_type: String,
    brand: Option<FacadeBrand>,
    location: Option<FacadeLocation>,
}

#[derive(Debug, Clone, Deserialize)]
struct FacadeBrand {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct FacadeLocation {
    #[serde(default)]
    city: String,
    #[serde(default)]
    state: String,
    #[serde(default)]
    country: String,
}

#[derive(Debug, Clone, Deserialize)]
struct FacadeResponse {
    data: FacadeJob,
}

pub struct EfinancialcareersScraper {
    lang: LanguageService,
}

impl EfinancialcareersScraper {
    #[must_use]
    pub fn new(lang: LanguageService) -> Self {
        Self { lang }
    }

    async fn ensure_efinancialcareers_tab(&self, browser: &Browser) -> Result<()> {
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
        Ok(())
    }

    pub async fn wait_for_jobs(page: &chromiumoxide::Page) -> Result<bool> {
        crate::browser::wait_for_with_challenge_recovery(
            page,
            SEARCH_RESULTS_CONTAINER_JS,
            None,
            None,
            None,
            None,
        )
        .await
    }

    async fn process_search_card(
        &self,
        db: &Db,
        card: &EfinancialcareersJobCard,
        total_jobs: usize,
        pause_ms: u64,
        state: &mut FetchState,
    ) -> Result<()> {
        if db
            .find_job_id(&Platform::Efinancialcareers, &card.external_id)
            .await?
            .is_some()
        {
            state.inc_existing();
            eprint!("{}", state.progress_line(Some(total_jobs), ""));
            return Ok(());
        }

        if db
            .is_rejected(&Platform::Efinancialcareers, &card.external_id)
            .await?
        {
            state.inc_skipped();
            eprint!("{}", state.progress_line(Some(total_jobs), ""));
            return Ok(());
        }

        sleep(Duration::from_millis(pause_ms)).await;

        let http = reqwest::Client::new();
        let detail = match self.fetch_detail(&http, &card.external_id).await {
            Ok(d) => d,
            Err(e) => {
                eprintln!(
                    "    Warning: failed to fetch detail for {}: {}",
                    card.external_id, e
                );
                return Ok(());
            }
        };

        let job = self.build_job(card, detail).await?;
        if let Some(job) = job {
            match db.upsert_job(&job).await? {
                UpsertResult::New(_) => state.inc_new(),
                UpsertResult::Updated(_) => {
                    state.inc_existing();
                }
                UpsertResult::Duplicate(_) => {
                    state.inc_existing();
                    db.mark_rejected(&Platform::Efinancialcareers, &card.external_id, "duplicate")
                        .await?;
                }
            }
        } else {
            state.inc_skipped();
            db.mark_rejected(
                &Platform::Efinancialcareers,
                &card.external_id,
                "non_english",
            )
            .await?;
        }

        eprint!("{}", state.progress_line(Some(total_jobs), &card.title));
        Ok(())
    }

    pub async fn scrape_page(page: &chromiumoxide::Page) -> Result<Vec<EfinancialcareersJobCard>> {
        let cards: Vec<EfinancialcareersJobCard> =
            page.evaluate(SCRAPE_CARDS_JS).await?.into_value()?;
        Ok(cards)
    }

    pub async fn scrape_total(page: &chromiumoxide::Page) -> Result<usize> {
        let total: Option<i64> = page.evaluate(SCRAPE_TOTAL_JS).await?.into_value()?;
        match total {
            Some(n) if n >= 0 => usize::try_from(n)
                .map_err(|_| anyhow::anyhow!("eFinancialCareers total exceeds usize")),
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

    /// Fetch job detail from the public job-branding-facade API.
    pub async fn fetch_detail(
        &self,
        http: &reqwest::Client,
        job_id: &str,
    ) -> Result<EfinancialcareersJobDetail> {
        let url = format!("https://job-branding-facade.efinancialcareers.com/job/{job_id}");
        let res = http
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await?;
        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            bail!("job detail fetch failed: {body}");
        }
        let facade: FacadeResponse = res.json().await?;
        let job = facade.data;

        let description = html::html_to_md(&job.description).unwrap_or_default();
        let posted_at = DateTime::parse_from_rfc3339(&job.posted_date)
            .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&Utc));
        let company = job.brand.map(|b| b.name).unwrap_or_default();
        let location = job
            .location
            .map(|l| {
                [l.city, l.state, l.country]
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        let employment_type = [job.position_type, job.employment_type]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" / ");

        Ok(EfinancialcareersJobDetail {
            company,
            location,
            employment_type,
            work_arrangement_type: job.work_arrangement_type,
            salary: job.salary,
            description,
            posted_at,
        })
    }

    async fn build_job(
        &self,
        card: &EfinancialcareersJobCard,
        detail: EfinancialcareersJobDetail,
    ) -> Result<Option<Job>> {
        let created_at = detail.posted_at;
        let salary = if detail.salary.is_empty() {
            card.salary.clone()
        } else {
            detail.salary.clone()
        };
        let budget = crate::extractors::budget::parse_efinancialcareers_budget(&salary)
            .map(|b| b.to_string())
            .or_else(|| Some(salary.clone()).filter(|b| !b.is_empty()));

        let remote = Self::is_remote(&detail);

        let job = Job {
            id: 0,
            platform: Platform::Efinancialcareers,
            external_id: card.external_id.clone(),
            title: card.title.clone(),
            url: card.url.clone(),
            budget,
            tags: Vec::new(),
            raw: Data::Efinancialcareers {
                detail: EfinancialcareersJobDetail {
                    salary: salary.clone(),
                    ..detail
                },
            },
            company: None,
            created_at,
            updated_at: Utc::now(),
            rating: Rating::Neutral,
            note: None,
            applied_at: None,
            remote,
        };
        let is_english = classify_language(&self.lang, &job).await?;
        if !is_english {
            return Ok(None);
        }
        Ok(Some(job))
    }

    fn is_remote(detail: &EfinancialcareersJobDetail) -> bool {
        matches!(
            detail.work_arrangement_type.to_lowercase().as_str(),
            "remote" | "temporarily_remote"
        ) || detail.location.to_lowercase().contains("remote")
            || detail.employment_type.to_lowercase().contains("remote")
    }
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
        url: &str,
        pause_ms: u64,
    ) -> Result<FetchState> {
        self.ensure_efinancialcareers_tab(browser).await?;

        let parsed = url::Url::parse(url)
            .map_err(|e| anyhow::anyhow!("invalid eFinancialCareers URL: {e}"))?;
        let host = parsed.host_str().unwrap_or_default();
        if !host.ends_with("efinancialcareers.com") {
            bail!("eFinancialCareers URL must be on efinancialcareers.com subdomain");
        }

        let page = browser.new_tab(url).await?;

        if !Self::wait_for_jobs(&page).await? {
            page.close().await.ok();
            bail!("eFinancialCareers search page did not load job cards");
        }

        let total_jobs = Self::scrape_total(&page).await?;
        if total_jobs == 0 {
            page.close().await.ok();
            return Ok(FetchState::new());
        }

        let mut processed_ids: HashSet<String> = HashSet::new();
        let mut state = FetchState::new();
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
                self.process_search_card(db, card, total_jobs, pause_ms, &mut state)
                    .await?;
            }

            if !Self::click_show_more(&page, pause_ms).await {
                break;
            }
        }

        page.close().await.ok();
        Ok(state)
    }
}

impl Default for EfinancialcareersScraper {
    fn default() -> Self {
        Self::new(LanguageService::new())
    }
}
