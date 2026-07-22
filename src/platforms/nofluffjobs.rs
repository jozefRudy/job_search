use super::html;
use crate::browser::{BrowserExt, host_of, wait_for, wait_for_element};
use crate::db::{Db, UpsertResult};
use crate::language::LanguageService;
use crate::models::{Data, Job, NoFluffJobDetail, Platform, Rating, classify_language};
use crate::platforms::{FetchState, PlatformClient};
use crate::term::CursorGuard;
use anyhow::{Result, bail};
use async_trait::async_trait;
use chromiumoxide::browser::Browser;
use chrono::Utc;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use tokio::time::{Duration, sleep};

const SCRAPE_CARDS_JS: &str = include_str!("nofluffjobs/scrape_cards.js");
const POSTING_LIST_ITEM_JS: &str = include_str!("nofluffjobs/posting_list_item.js");
const CLICK_LOAD_MORE_JS: &str = include_str!("nofluffjobs/click_load_more.js");
const COUNT_CARDS_JS: &str = include_str!("nofluffjobs/count_cards.js");
const GET_TOTAL_RESULTS_JS: &str = include_str!("nofluffjobs/get_total_results.js");

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
    client: Client,
    lang: LanguageService,
}

const API_BASE: &str = "https://nofluffjobs.com/api";

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

        let description = html::html_to_md(&raw.details.description).unwrap_or_default();

        let requirements = html::html_to_md(&raw.requirements.description).unwrap_or_default();

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
        url: &str,
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
        if !self.is_logged_in(browser).await? {
            bail!("NoFluffJobs requires a logged-in nofluffjobs.com session");
        }

        self.fetch_jobs_via_browser(browser, db, url, pause_ms)
            .await
    }
}

impl NoFluffJobsScraper {
    #[must_use]
    pub fn new(lang: LanguageService) -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (compatible; JobSearch/1.0)")
                .build()
                .unwrap_or_else(|_| Client::new()),
            lang,
        }
    }

    /// Best-effort check that a user profile/auth cookie exists.
    async fn is_logged_in(&self, browser: &Browser) -> Result<bool> {
        let cookies = browser.get_cookies().await.unwrap_or_default();
        Ok(cookies
            .iter()
            .any(|c| c.name == "nfj_at" || c.name == "nfj_session"))
    }

    async fn process_search_card(
        &self,
        db: &Db,
        card: &NofluffJobCard,
        total_results: Option<usize>,
        pause_ms: u64,
        state: &mut FetchState,
    ) -> Result<()> {
        let platform = Platform::NoFluffJobs;
        if db
            .find_job_id(&platform, &card.external_id)
            .await?
            .is_some()
        {
            state.inc_existing();
            eprint!("{}", state.progress_line(total_results, ""));
            return Ok(());
        }

        if db.is_rejected(&platform, &card.external_id).await? {
            state.inc_skipped();
            eprint!("{}", state.progress_line(total_results, ""));
            return Ok(());
        }

        sleep(Duration::from_millis(pause_ms)).await;

        match self.fetch_detail(&card.external_id).await {
            Ok(detail) => {
                let posted = detail.posted_at;
                let budget = card.budget.as_ref().and_then(|b| {
                    let normalized = b.replace(['\u{00a0}', '\u{2007}', '\u{202f}'], " ");
                    if normalized.trim() == "Salary Match" {
                        None
                    } else {
                        crate::extractors::budget::parse_nofluff_budget(b).map(|b| b.to_string())
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
                };
                let is_english = classify_language(&self.lang, &job).await?;
                if is_english {
                    match db.upsert_job(&job).await? {
                        UpsertResult::New(_) => state.inc_new(),
                        UpsertResult::Updated(_) | UpsertResult::Duplicate(_) => {
                            state.inc_existing();
                        }
                    }
                } else {
                    state.inc_skipped();
                }
            }
            Err(e) => {
                eprintln!(
                    "    Warning: failed to fetch detail for {}: {}",
                    card.external_id, e
                );
            }
        }

        eprint!("{}", state.progress_line(total_results, &card.external_id));
        Ok(())
    }

    /// Scrape job cards from `NoFluffJobs` search page via browser.
    /// The configured URL controls the search; this method clicks "See more offers".
    pub async fn fetch_jobs_via_browser(
        &self,
        browser: &Browser,
        db: &Db,
        url: &str,
        pause_ms: u64,
    ) -> Result<FetchState> {
        let parsed =
            url::Url::parse(url).map_err(|e| anyhow::anyhow!("invalid NoFluffJobs URL: {e}"))?;
        let host = parsed.host_str().unwrap_or_default();
        if !host.ends_with("nofluffjobs.com") {
            bail!("NoFluffJobs URL must be on nofluffjobs.com subdomain");
        }

        let page = browser.new_tab(url).await?;

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
                self.process_search_card(db, card, total_results, pause_ms, &mut state)
                    .await?;
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
            .query(&[("salaryPeriod", "month")])
            .send()
            .await?
            .json()
            .await?;

        envelope.try_into()
    }
}

impl Default for NoFluffJobsScraper {
    fn default() -> Self {
        Self::new(LanguageService::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
