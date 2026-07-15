use crate::browser::{BrowserExt, wait_for_with_challenge_recovery};
use crate::db::Db;
use crate::models::{Budget, Data, Job, LinkedInJobDetail, Platform, Rating};
use crate::platforms::{FetchState, PlatformClient};
use anyhow::{Result, bail};
use async_trait::async_trait;
use chromiumoxide::Page;
use chromiumoxide::browser::Browser;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use url::Url;

const VOYAGER_FETCH_JS: &str = include_str!("linkedin/voyager_fetch.js");
const JOB_DETAIL_FETCH_JS: &str = include_str!("linkedin/job_detail_fetch.js");

// Search filters
const GEO_ID: &str = "92000000";
const INDUSTRY_ID: &str = "4";
const TITLE_IDS: &[&str] = &["9", "25201", "30128"];
const WORKPLACE_TYPE_REMOTE: &str = "2";
const SORT_BY_DATE: &str = "DD";

// URL routes
const LINKEDIN_JOBS_BASE_URL: &str = "https://www.linkedin.com/jobs/search/";
const VOYAGER_BASE_URL: &str = "https://www.linkedin.com/voyager/api/voyagerJobsDashJobCards";
const VOYAGER_GRAPHQL_BASE_URL: &str = "https://www.linkedin.com/voyager/api/graphql";
const JOB_VIEW_BASE_URL: &str = "https://www.linkedin.com/jobs/view/";
const JOB_POSTING_URN_PREFIX: &str = "urn:li:fsd_jobPosting:";

// Voyager API identifiers
const VOYAGER_DECORATION_ID: &str =
    "com.linkedin.voyager.dash.deco.jobs.search.JobSearchCardsCollection-220";
const VOYAGER_JOB_DETAIL_QUERY_ID: &str =
    "voyagerJobsDashJobPostingDetailSections.77cb64956921ef397a36de4f7f8bce47";
const VOYAGER_JOB_POSTING_QUERY_ID: &str =
    "voyagerJobsDashJobPostings.891aed7916d7453a37e4bbf5f1f60de4";
const VOYAGER_JOB_DETAIL_CARDS: &[&str] = &[
    "TOP_CARD",
    "HOW_YOU_FIT_CARD",
    "JOB_DESCRIPTION_CARD",
    "SALARY_CARD",
];

// JS placeholders
const VOYAGER_URL_PLACEHOLDER: &str = "__VOYAGER_URL__";
const JOB_CONFIG_PLACEHOLDER: &str = "__JOB_CONFIG__";

// Pagination
const PAGE_SIZE: usize = 25;

// DOM checks
const JOB_CARD_PRESENT_JS: &str = "!!document.querySelector('[data-job-id]')";

pub struct LinkedInScraper {
    // TODO(phase1): Parse search parameters from configured URL at construction; remove CLI `since_days`.
    since_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedInJobCard {
    pub id: String,
    pub title: String,
    #[serde(rename = "listedAt")]
    pub listed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoyagerCardsResult {
    pub cards: Vec<LinkedInJobCard>,
    pub total: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct LinkedInJobDetailJs {
    company: String,
    location: String,
    employment_type: String,
    job_function: String,
    industries: String,
    description: String,
    salary: String,
    posted_at: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JobDetailConfig {
    base_url: String,
    detail_query_id: String,
    job_query_id: String,
    job_posting_urn: String,
    card_section_types: Vec<String>,
}

impl From<LinkedInJobDetailJs> for LinkedInJobDetail {
    fn from(raw: LinkedInJobDetailJs) -> Self {
        Self {
            company: raw.company,
            location: raw.location,
            employment_type: raw.employment_type,
            job_function: raw.job_function,
            industries: raw.industries,
            description: raw.description,
            salary: raw.salary,
            posted_at: DateTime::from_timestamp_millis(raw.posted_at).unwrap_or_else(Utc::now),
        }
    }
}

impl LinkedInScraper {
    #[must_use]
    pub fn new(since_days: u32) -> Self {
        Self { since_days }
    }

    #[must_use]
    pub fn build_search_url(&self) -> String {
        let seconds = u64::from(self.since_days) * 86400;
        let params = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("f_I", INDUSTRY_ID)
            .append_pair("f_T", &TITLE_IDS.join(","))
            .append_pair("f_TPR", &format!("r{seconds}"))
            .append_pair("f_WT", WORKPLACE_TYPE_REMOTE)
            .append_pair("geoId", GEO_ID)
            .append_pair("origin", "JOB_SEARCH_PAGE_JOB_FILTER")
            .append_pair("sortBy", SORT_BY_DATE)
            .finish();
        let mut url = Url::parse(LINKEDIN_JOBS_BASE_URL).expect("valid linkedin jobs base URL");
        url.set_query(Some(&params));
        url.to_string()
    }

    async fn ensure_linkedin_tab(&self, browser: &Browser) -> Result<()> {
        let hosts: Vec<_> = browser
            .get_page_urls()
            .await?
            .into_iter()
            .filter_map(|u| crate::browser::host_of(&u))
            .collect();
        if !hosts.iter().any(|h| h.contains("linkedin.com")) {
            bail!("LinkedIn requires open linkedin.com tab in Brave");
        }
        Ok(())
    }

    pub async fn wait_for_jobs(page: &Page) -> Result<bool> {
        wait_for_with_challenge_recovery(page, JOB_CARD_PRESENT_JS, None, Some(60), None, None)
            .await
    }

    pub async fn fetch_page(&self, page: &Page, start: usize) -> Result<VoyagerCardsResult> {
        let voyager_url = build_voyager_search_url(start, self.since_days);
        let js = VOYAGER_FETCH_JS.replace(VOYAGER_URL_PLACEHOLDER, &voyager_url);
        let result: VoyagerCardsResult = page.evaluate(js.as_str()).await?.into_value()?;
        Ok(result)
    }

    pub async fn scrape_page(&self, page: &Page) -> Result<Vec<LinkedInJobCard>> {
        self.fetch_page(page, 0).await.map(|r| r.cards)
    }
}

#[async_trait]
impl PlatformClient for LinkedInScraper {
    fn name(&self) -> &'static str {
        "linkedin"
    }

    async fn fetch_with_browser(
        &self,
        browser: &Browser,
        db: &Db,
        _query: &str,
        pause_ms: u64,
    ) -> Result<FetchState> {
        self.ensure_linkedin_tab(browser).await?;

        // TODO(phase1): Validate `url` host is linkedin.com subdomain before using it.
        // TODO(phase1): Use `url: &str` as the LinkedIn search URL. Parse only these query params:
        //   geoId, f_T, f_I, f_TPR, f_WT. Bail on unknown params. Ignore sortBy (hardcoded to most recent).
        // Translate them into the Voyager search query. Voyager remains used for cards and details.
        let search_url = self.build_search_url();
        let page = browser.new_tab(&search_url).await?;
        let loaded = Self::wait_for_jobs(&page).await?;
        if !loaded {
            bail!("LinkedIn search page did not load. Ensure you are logged in at linkedin.com.");
        }

        let lang = crate::language::LanguageService::new();
        let mut no_cards_fetched = 0;
        let mut state = FetchState::new();
        let _guard = crate::term::CursorGuard::new();

        'pages: loop {
            let result = self.fetch_page(&page, no_cards_fetched).await?;
            let total = result.total as usize;
            let cards = result.cards;
            let card_count = cards.len();

            for card in cards {
                if db
                    .find_job_id(&Platform::LinkedIn, &card.id)
                    .await?
                    .is_some()
                {
                    state.inc_existing();
                    eprint!("{}", state.progress_line(Some(total), ""));
                    break 'pages;
                }

                let job_id: u64 = card
                    .id
                    .parse()
                    .map_err(|e| anyhow::anyhow!("invalid LinkedIn job id {e}"))?;

                let detail = match fetch_job_detail(&page, job_id).await {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!(
                            "    Warning: failed to fetch detail for {}: {e}",
                            card.title
                        );
                        continue;
                    }
                };

                let company = Some(detail.company.clone()).filter(|c| !c.is_empty());
                let description = Some(detail.description.clone()).filter(|d| !d.is_empty());
                let posted_at = card
                    .listed_at
                    .and_then(DateTime::from_timestamp_millis)
                    .unwrap_or_else(Utc::now);

                let job = Job {
                    id: 0,
                    platform: Platform::LinkedIn,
                    external_id: card.id,
                    title: card.title,
                    description,
                    url: format!("{JOB_VIEW_BASE_URL}{job_id}/"),
                    budget: Budget::parse(&detail.salary, Some("yr")).map(|b| b.to_string()),
                    tags: Vec::new(),
                    raw: Data::LinkedIn { detail },
                    company,
                    created_at: posted_at,
                    updated_at: Utc::now(),
                    note: None,
                    rating: Rating::Neutral,
                    applied_at: None,
                    remote: true,
                };
                let is_english = crate::models::classify_language(&lang, &job).await?;
                if is_english {
                    db.upsert_job(&job).await?;
                    state.inc_new();
                    eprint!("{}", state.progress_line(Some(total), &job.title));
                }
                sleep(Duration::from_millis(pause_ms)).await;
            }

            if card_count == 0 || no_cards_fetched + card_count >= total {
                break;
            }
            no_cards_fetched += card_count;
            sleep(Duration::from_millis(pause_ms)).await;
        }

        page.close().await.ok();
        Ok(state)
    }
}

fn build_voyager_search_url(start: usize, since_days: u32) -> String {
    let query = build_voyager_query(since_days);
    format!(
        "{VOYAGER_BASE_URL}?decorationId={VOYAGER_DECORATION_ID}&count={PAGE_SIZE}&q=jobSearch&query={query}&start={start}"
    )
}

fn build_voyager_query(since_days: u32) -> String {
    let seconds = u64::from(since_days) * 86400;
    let titles = TITLE_IDS.join(",");
    format!(
        "(origin:JOB_SEARCH_PAGE_JOB_FILTER,locationUnion:(geoId:{GEO_ID}),selectedFilters:(sortBy:List({SORT_BY_DATE}),industry:List({INDUSTRY_ID}),title:List({titles}),timePostedRange:List(r{seconds}),workplaceType:List({WORKPLACE_TYPE_REMOTE})))"
    )
}

pub async fn fetch_job_detail(page: &Page, job_id: u64) -> Result<LinkedInJobDetail> {
    let job_posting_urn = format!("{JOB_POSTING_URN_PREFIX}{job_id}");
    let config = JobDetailConfig {
        base_url: VOYAGER_GRAPHQL_BASE_URL.to_string(),
        detail_query_id: VOYAGER_JOB_DETAIL_QUERY_ID.to_string(),
        job_query_id: VOYAGER_JOB_POSTING_QUERY_ID.to_string(),
        job_posting_urn,
        card_section_types: VOYAGER_JOB_DETAIL_CARDS
            .iter()
            .copied()
            .map(String::from)
            .collect(),
    };
    let config_json = serde_json::to_string(&config)?;
    let js = JOB_DETAIL_FETCH_JS.replace(JOB_CONFIG_PLACEHOLDER, &config_json);
    let result: LinkedInJobDetailJs = page.evaluate(js.as_str()).await?.into_value()?;
    Ok(result.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    // TODO(phase1): Update tests to assert URL parsing or keep Voyager query builder tests.
    // TODO(phase1): Add integration test for LinkedIn URL param -> Voyager query translation.
    use std::collections::HashMap;

    fn query_pairs(url: &str) -> HashMap<String, String> {
        Url::parse(url)
            .expect("valid URL")
            .query_pairs()
            .into_owned()
            .collect()
    }

    #[test]
    fn test_build_search_url_defaults() {
        let scraper = LinkedInScraper::new(30);
        let pairs = query_pairs(&scraper.build_search_url());
        assert_eq!(pairs["f_I"], "4");
        assert_eq!(pairs["f_T"], "9,25201,30128");
        assert_eq!(pairs["f_TPR"], "r2592000");
        assert_eq!(pairs["f_WT"], "2");
        assert_eq!(pairs["geoId"], GEO_ID);
        assert_eq!(pairs["sortBy"], SORT_BY_DATE);
    }

    #[test]
    fn test_build_search_url_since_days() {
        let scraper = LinkedInScraper::new(7);
        let pairs = query_pairs(&scraper.build_search_url());
        assert_eq!(pairs["f_TPR"], "r604800");
    }

    #[test]
    fn test_build_voyager_search_url() {
        let url = build_voyager_search_url(0, 30);
        let pairs = query_pairs(&url);
        assert_eq!(pairs["decorationId"], VOYAGER_DECORATION_ID);
        assert_eq!(pairs["count"], "25");
        assert_eq!(pairs["q"], "jobSearch");
        assert_eq!(pairs["start"], "0");
        let query = &pairs["query"];
        assert!(query.contains("timePostedRange:List(r2592000)"));
        assert!(query.contains("workplaceType:List(2)"));
        assert!(query.contains("title:List(9,25201,30128)"));
    }
}
