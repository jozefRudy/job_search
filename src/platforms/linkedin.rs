use crate::browser::{BrowserExt, wait_for_with_challenge_recovery};
use crate::db::Db;
use crate::models::{Budget, Data, Job, LinkedInJobDetail, Platform, Rating};
use crate::platforms::{FetchState, PlatformClient};
use crate::term::CursorGuard;
use anyhow::{Result, bail};
use async_trait::async_trait;
use chromiumoxide::Page;
use chromiumoxide::browser::Browser;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio::time::sleep;
use url::Url;

const VOYAGER_FETCH_JS: &str = include_str!("linkedin/voyager_fetch.js");
const JOB_DETAIL_FETCH_JS: &str = include_str!("linkedin/job_detail_fetch.js");

const VOYAGER_BASE_URL: &str = "https://www.linkedin.com/voyager/api/voyagerJobsDashJobCards";
const VOYAGER_GRAPHQL_BASE_URL: &str = "https://www.linkedin.com/voyager/api/graphql";
const JOB_VIEW_BASE_URL: &str = "https://www.linkedin.com/jobs/view/";
const JOB_POSTING_URN_PREFIX: &str = "urn:li:fsd_jobPosting:";

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

const JS_URL_PLACEHOLDER: &str = "__VOYAGER_URL__";
const JOB_CONFIG_PLACEHOLDER: &str = "__JOB_CONFIG__";

const PAGE_SIZE: usize = 100;
const JOB_CARD_PRESENT_JS: &str = "!!document.querySelector('[data-job-id]')";

const DEFAULT_GEO_ID: &str = "92000000";
const DEFAULT_INDUSTRY_ID: &str = "4";
const DEFAULT_TITLE_IDS: &[&str] = &["9", "25201", "30128"];
const DEFAULT_WORKPLACE_TYPE: &str = "2";
const DEFAULT_SORT_BY: &str = "DD";
const DEFAULT_TIME_POSTED_RANGE: &str = "r2592000";

const SUPPORTED_PARAMS: &[&str] = &["geoId", "f_T", "f_I", "f_WT", "f_TPR", "sortBy"];

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

#[derive(Debug, Clone)]
struct LinkedInParams {
    geo_id: String,
    title_ids: Vec<String>,
    industry_ids: Vec<String>,
    workplace_types: Vec<String>,
    time_posted_range: String,
}

impl Default for LinkedInParams {
    fn default() -> Self {
        Self {
            geo_id: DEFAULT_GEO_ID.to_string(),
            title_ids: DEFAULT_TITLE_IDS
                .iter()
                .copied()
                .map(String::from)
                .collect(),
            industry_ids: vec![DEFAULT_INDUSTRY_ID.to_string()],
            workplace_types: vec![DEFAULT_WORKPLACE_TYPE.to_string()],
            time_posted_range: DEFAULT_TIME_POSTED_RANGE.to_string(),
        }
    }
}

impl LinkedInParams {
    fn parse_url(url: &str) -> Result<Self> {
        let parsed = Url::parse(url).map_err(|e| anyhow::anyhow!("invalid LinkedIn URL: {e}"))?;
        let query: HashMap<String, String> = parsed.query_pairs().into_owned().collect();

        for key in query.keys() {
            if !SUPPORTED_PARAMS.contains(&key.as_str()) {
                bail!("unsupported LinkedIn URL parameter: {key}");
            }
        }

        let geo_id = query
            .get("geoId")
            .map_or_else(|| DEFAULT_GEO_ID.to_string(), String::clone);
        let title_ids = query.get("f_T").map_or_else(
            || {
                DEFAULT_TITLE_IDS
                    .iter()
                    .copied()
                    .map(String::from)
                    .collect()
            },
            |s| s.split(',').map(String::from).collect(),
        );
        let industry_ids = query.get("f_I").map_or_else(
            || vec![DEFAULT_INDUSTRY_ID.to_string()],
            |s| s.split(',').map(String::from).collect(),
        );
        let workplace_types = query.get("f_WT").map_or_else(
            || vec![DEFAULT_WORKPLACE_TYPE.to_string()],
            |s| s.split(',').map(String::from).collect(),
        );
        let time_posted_range = query
            .get("f_TPR")
            .map_or_else(|| DEFAULT_TIME_POSTED_RANGE.to_string(), String::clone);

        if !time_posted_range.starts_with('r') {
            bail!("f_TPR must start with 'r' (e.g. r2592000)");
        }

        Ok(Self {
            geo_id,
            title_ids,
            industry_ids,
            workplace_types,
            time_posted_range,
        })
    }

    fn build_voyager_query(&self) -> String {
        format!(
            "(origin:JOB_SEARCH_PAGE_JOB_FILTER,locationUnion:(geoId:{}),selectedFilters:(sortBy:List({}),industry:List({}),title:List({}),timePostedRange:List({}),workplaceType:List({})))",
            self.geo_id,
            DEFAULT_SORT_BY,
            self.industry_ids.join(","),
            self.title_ids.join(","),
            self.time_posted_range,
            self.workplace_types.join(",")
        )
    }

    fn build_voyager_search_url(&self, start: usize) -> String {
        let query = self.build_voyager_query();
        format!(
            "{VOYAGER_BASE_URL}?decorationId={VOYAGER_DECORATION_ID}&count={PAGE_SIZE}&q=jobSearch&query={query}&start={start}"
        )
    }
}

pub struct LinkedInScraper {
    params: LinkedInParams,
}

impl LinkedInScraper {
    #[must_use]
    pub fn new(url: &str) -> Self {
        let params = LinkedInParams::parse_url(url).unwrap_or_default();
        Self { params }
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
        let voyager_url = self.params.build_voyager_search_url(start);
        let js = VOYAGER_FETCH_JS.replace(JS_URL_PLACEHOLDER, &voyager_url);
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
        url: &str,
        pause_ms: u64,
    ) -> Result<FetchState> {
        self.ensure_linkedin_tab(browser).await?;

        let parsed = Url::parse(url).map_err(|e| anyhow::anyhow!("invalid LinkedIn URL: {e}"))?;
        let host = parsed.host_str().unwrap_or_default();
        if !host.ends_with("linkedin.com") {
            bail!("LinkedIn URL must be on linkedin.com subdomain");
        }

        let page = browser.new_tab(url).await?;
        let loaded = Self::wait_for_jobs(&page).await?;
        if !loaded {
            bail!("LinkedIn search page did not load. Ensure you are logged in at linkedin.com.");
        }

        let lang = crate::language::LanguageService::new();
        let mut no_cards_fetched = 0;
        let mut state = FetchState::new();
        let _guard = CursorGuard::new();

        loop {
            let result = self.fetch_page(&page, no_cards_fetched).await?;
            let total = result.total as usize;
            let cards = result.cards;
            let card_count = cards.len();

            let ids: Vec<String> = cards.iter().map(|c| c.id.clone()).collect();
            let new_ids: HashSet<String> = db
                .filter_new(&Platform::LinkedIn, &ids)
                .await?
                .into_iter()
                .collect();
            let new_cards: Vec<LinkedInJobCard> = cards
                .into_iter()
                .filter(|c| new_ids.contains(&c.id))
                .collect();
            let existing_count = card_count.saturating_sub(new_cards.len());
            state.inc_existing_n(existing_count);

            for card in new_cards {
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

    #[test]
    fn test_parse_url_defaults() {
        let params = LinkedInParams::parse_url("https://www.linkedin.com/jobs/search/").unwrap();
        assert_eq!(params.geo_id, DEFAULT_GEO_ID);
        assert_eq!(params.title_ids, DEFAULT_TITLE_IDS);
        assert_eq!(params.industry_ids, vec![DEFAULT_INDUSTRY_ID]);
        assert_eq!(params.workplace_types, vec![DEFAULT_WORKPLACE_TYPE]);
        assert_eq!(params.time_posted_range, DEFAULT_TIME_POSTED_RANGE);
    }

    #[test]
    fn test_parse_url_custom_params() {
        let url = "https://www.linkedin.com/jobs/search/?f_TPR=r604800&f_WT=2&geoId=12345&f_T=9,25201&f_I=4";
        let params = LinkedInParams::parse_url(url).unwrap();
        assert_eq!(params.geo_id, "12345");
        assert_eq!(params.title_ids, vec!["9", "25201"]);
        assert_eq!(params.industry_ids, vec!["4"]);
        assert_eq!(params.workplace_types, vec!["2"]);
        assert_eq!(params.time_posted_range, "r604800");
    }

    #[test]
    fn test_parse_url_rejects_unknown_param() {
        let url = "https://www.linkedin.com/jobs/search/?f_T=9&foo=bar";
        assert!(LinkedInParams::parse_url(url).is_err());
    }

    #[test]
    fn test_build_voyager_query() {
        let params = LinkedInParams::default();
        let query = params.build_voyager_query();
        assert!(query.contains(&format!("geoId:{DEFAULT_GEO_ID}")));
        assert!(query.contains("timePostedRange:List(r2592000)"));
        assert!(query.contains("workplaceType:List(2)"));
        assert!(query.contains("title:List(9,25201,30128)"));
    }

    #[test]
    fn test_build_voyager_search_url() {
        let params = LinkedInParams::default();
        let url = params.build_voyager_search_url(0);
        assert!(url.starts_with(VOYAGER_BASE_URL));
        assert!(url.contains(&format!("decorationId={VOYAGER_DECORATION_ID}")));
        assert!(url.contains("count="));
    }
}
