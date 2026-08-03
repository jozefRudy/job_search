//! Work at a Startup (Y Combinator) platform client.
//!
//! Backend: public Algolia index `WaaSPublicCompanyJob_created_at_desc_production`
//! (app id `45BWZJ1SGC`). The Algolia secured API key embeds a per-session
//! `userToken` and is only issued to logged-in sessions, so fetching runs inside
//! a workatastartup.com browser tab: JS extracts the full key URL from
//! `performance.getEntriesByType('resource')` and paginates `/1/indexes/*/queries`.

use crate::browser::{BrowserExt, host_of};
use crate::db::{Db, UpsertResult};
use crate::models::{Data, Job, Platform, Rating, WorkAtStartupJobDetail};
use crate::platforms::{FetchState, PlatformClient};
use crate::term::CursorGuard;
use anyhow::{Result, bail};
use async_trait::async_trait;
use chromiumoxide::Page;
use chromiumoxide::browser::Browser;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, sleep};

const QUERY_ALGOLIA_JS: &str = include_str!("workatastartup/query_algolia.js");
const CONFIG_PLACEHOLDER: &str = "__CONFIG__";

const ALGOLIA_INDEX: &str = "WaaSPublicCompanyJob_created_at_desc_production";
const NO_KEY_HINT: &str = "no algolia key";
const KEY_WAIT_ATTEMPTS: u32 = 20;

/// Filters parsed from a workatastartup.com/companies URL, translated to an
/// Algolia `filters` expression.
#[derive(Debug, Clone, Default)]
pub struct WaasFilters {
    pub role: Option<String>,
    pub remote: Option<String>,
    pub us_visa_not_required: bool,
    pub job_type: Option<String>,
}

impl WaasFilters {
    /// Parse filters from workatastartup.com/companies query params.
    /// Bails on non-workatastartup.com host.
    pub fn from_url(url: &str) -> Result<Self> {
        let parsed = url::Url::parse(url)?;
        let host = parsed.host_str().unwrap_or_default();
        if !host.ends_with("workatastartup.com") {
            bail!("Workatastartup URL must be on workatastartup.com");
        }
        let param = |key: &str| {
            parsed
                .query_pairs()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.into_owned())
                .filter(|v| v != "any")
        };
        Ok(Self {
            role: param("role"),
            remote: param("remote"),
            us_visa_not_required: param("usVisaNotRequired").is_some_and(|v| v == "true"),
            job_type: param("jobType"),
        })
    }

    /// Algolia filter expression, e.g.
    /// `(role:eng) AND (remote:only) AND (us_visa_required:none OR us_visa_required:possible)`.
    #[must_use]
    pub fn to_algolia(&self) -> String {
        let mut clauses: Vec<String> = Vec::new();
        if let Some(role) = &self.role {
            clauses.push(format!("(role:{role})"));
        }
        if let Some(remote) = &self.remote {
            clauses.push(format!("(remote:{remote})"));
        }
        if self.us_visa_not_required {
            clauses.push("(us_visa_required:none OR us_visa_required:possible)".to_string());
        }
        if let Some(job_type) = &self.job_type {
            clauses.push(format!("(job_type:{job_type})"));
        }
        clauses.join(" AND ")
    }
}

/// Algolia hit from the `WaaSPublicCompanyJob` index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawWaasHit {
    #[serde(rename = "objectID")]
    pub object_id: String,
    pub title: String,
    pub description: String,
    pub job_type: String,
    pub remote: String,
    pub role: String,
    #[serde(default)]
    pub eng_type: Vec<String>,
    pub min_experience: Option<u32>,
    pub created_at: String,
    #[serde(default)]
    pub skills: Vec<String>,
    pub has_salary: Option<bool>,
    pub has_equity: Option<bool>,
    pub has_interview_process: Option<bool>,
    pub us_visa_required: String,
    pub company_name: String,
    pub company_website: Option<String>,
    pub company_description: Option<String>,
    pub company_parent_sector: Option<String>,
    pub company_team_size: Option<u32>,
    pub company_waas_stage: Option<String>,
    pub search_path: String,
}

#[derive(Debug, Deserialize)]
pub struct AlgoliaPage {
    #[serde(default)]
    pub hits: Vec<RawWaasHit>,
    #[serde(default, rename = "nbPages")]
    pub nb_pages: u32,
    pub error: Option<String>,
}

pub struct WorkatastartupScraper;

impl WorkatastartupScraper {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Bail unless an open tab host contains workatastartup.com (user must be
    /// logged in — the Algolia secured key is only issued to sessions).
    async fn ensure_waas_tab(&self, browser: &Browser) -> Result<()> {
        let page_hosts: Vec<_> = browser
            .get_page_urls()
            .await?
            .into_iter()
            .filter_map(|u| host_of(&u))
            .collect();
        if !page_hosts.iter().any(|h| h.contains("workatastartup.com")) {
            bail!("Workatastartup requires open workatastartup.com tab in browser (logged in)");
        }
        Ok(())
    }

    /// Evaluate QUERY_ALGOLIA_JS: extract secured key URL from performance
    /// resource entries, POST one page of hits. The key appears only after the
    /// tab's own search request fires, so missing key is retried.
    pub async fn fetch_page(
        &self,
        page: &Page,
        filters: &str,
        page_num: u32,
    ) -> Result<AlgoliaPage> {
        let config = serde_json::json!({
            "filters": filters,
            "page": page_num,
            "indexName": ALGOLIA_INDEX,
        });
        let js = QUERY_ALGOLIA_JS.replace(CONFIG_PLACEHOLDER, &config.to_string());
        for attempt in 0..KEY_WAIT_ATTEMPTS {
            let result: AlgoliaPage = page.evaluate(js.as_str()).await?.into_value()?;
            match result.error {
                Some(e) if e.contains(NO_KEY_HINT) && attempt + 1 < KEY_WAIT_ATTEMPTS => {
                    sleep(Duration::from_millis(500)).await;
                }
                Some(e) => bail!("algolia query failed: {e}"),
                None => return Ok(result),
            }
        }
        unreachable!()
    }

    /// Map an Algolia hit to a `Job` (external_id = objectID, url = search_path,
    /// remote = hit.remote == "only").
    #[must_use]
    pub fn hit_to_job(hit: &RawWaasHit) -> Job {
        let created_at = DateTime::parse_from_rfc3339(&hit.created_at)
            .map_or_else(|_| Utc::now(), |d| d.with_timezone(&Utc));
        Job {
            id: 0,
            platform: Platform::Workatastartup,
            external_id: hit.object_id.clone(),
            title: hit.title.clone(),
            url: hit.search_path.clone(),
            budget: None,
            tags: hit.skills.clone(),
            raw: Data::Workatastartup {
                detail: Self::hit_to_detail(hit),
            },
            company: Some(hit.company_name.clone()),
            created_at,
            updated_at: Utc::now(),
            note: None,
            rating: Rating::Neutral,
            applied_at: None,
            remote: hit.remote == "only",
        }
    }

    #[must_use]
    fn hit_to_detail(hit: &RawWaasHit) -> WorkAtStartupJobDetail {
        WorkAtStartupJobDetail {
            description: hit.description.clone(),
            job_type: hit.job_type.clone(),
            remote: hit.remote.clone(),
            role: hit.role.clone(),
            eng_type: hit.eng_type.clone(),
            min_experience: hit.min_experience,
            skills: hit.skills.clone(),
            has_salary: hit.has_salary.unwrap_or(false),
            has_equity: hit.has_equity.unwrap_or(false),
            has_interview_process: hit.has_interview_process.unwrap_or(false),
            us_visa_required: hit.us_visa_required.clone(),
            company_name: hit.company_name.clone(),
            company_website: hit.company_website.clone(),
            company_description: hit.company_description.clone(),
            company_parent_sector: hit.company_parent_sector.clone(),
            company_team_size: hit.company_team_size,
            company_waas_stage: hit.company_waas_stage.clone(),
        }
    }
}

impl Default for WorkatastartupScraper {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl PlatformClient for WorkatastartupScraper {
    fn name(&self) -> &'static str {
        "workatastartup"
    }

    async fn fetch_with_browser(
        &self,
        browser: &Browser,
        db: &Db,
        url: &str,
        pause_ms: u64,
    ) -> Result<FetchState> {
        self.ensure_waas_tab(browser).await?;
        let filters = WaasFilters::from_url(url)?.to_algolia();

        let page = browser.new_tab(url).await?;
        let mut state = FetchState::new();
        let _guard = CursorGuard::new();

        let mut page_num = 0u32;
        loop {
            let result = self.fetch_page(&page, &filters, page_num).await?;
            if result.hits.is_empty() {
                break;
            }

            for hit in &result.hits {
                if db
                    .job_updated_at(&Platform::Workatastartup, &hit.object_id)
                    .await?
                    .is_some()
                {
                    state.inc_existing();
                } else {
                    let job = Self::hit_to_job(hit);
                    match db.upsert_job(&job).await? {
                        UpsertResult::New(_) => state.inc_new(),
                        UpsertResult::Updated(_) | UpsertResult::Duplicate(_) => {
                            state.inc_existing();
                        }
                    }
                }
                eprint!("{}", state.progress_line(None, &hit.title));
            }

            page_num += 1;
            if page_num >= result.nb_pages {
                break;
            }
            sleep(Duration::from_millis(pause_ms)).await;
        }
        page.close().await.ok();
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_from_url_parses_filters() {
        let f = WaasFilters::from_url(
            "https://www.workatastartup.com/companies?role=eng&remote=only&usVisaNotRequired=true&jobType=any&sortBy=created_desc",
        )
        .unwrap();
        assert_eq!(f.role.as_deref(), Some("eng"));
        assert_eq!(f.remote.as_deref(), Some("only"));
        assert!(f.us_visa_not_required);
        assert_eq!(f.job_type, None);
    }

    #[test]
    fn test_from_url_rejects_foreign_host() {
        assert!(WaasFilters::from_url("https://example.com/companies?role=eng").is_err());
    }

    #[test]
    fn test_to_algolia_full() {
        let f = WaasFilters {
            role: Some("eng".to_string()),
            remote: Some("only".to_string()),
            us_visa_not_required: true,
            job_type: None,
        };
        assert_eq!(
            f.to_algolia(),
            "(role:eng) AND (remote:only) AND (us_visa_required:none OR us_visa_required:possible)"
        );
    }

    #[test]
    fn test_to_algolia_empty() {
        assert_eq!(WaasFilters::default().to_algolia(), "");
    }

    #[test]
    fn test_hit_to_job_maps_fields() {
        let json = r#"{
            "objectID": "102480",
            "title": "Backend Engineer",
            "description": "Build stuff",
            "job_type": "fulltime",
            "remote": "only",
            "role": "eng",
            "eng_type": ["be"],
            "min_experience": 2,
            "created_at": "2026-08-03T00:13:41.025Z",
            "skills": ["rust", "postgres"],
            "has_salary": true,
            "has_equity": false,
            "has_interview_process": true,
            "us_visa_required": "none",
            "company_name": "VectorShift",
            "company_website": "https://www.vectorshift.ai",
            "company_description": null,
            "company_parent_sector": "B2B Software and Services",
            "company_team_size": 30,
            "company_waas_stage": "seed",
            "search_path": "https://www.ycombinator.com/companies/vectorshift/jobs/abc"
        }"#;
        let hit: RawWaasHit = serde_json::from_str(json).unwrap();
        let job = WorkatastartupScraper::hit_to_job(&hit);
        assert_eq!(job.external_id, "102480");
        assert_eq!(job.platform, Platform::Workatastartup);
        assert_eq!(job.company.as_deref(), Some("VectorShift"));
        assert!(job.remote);
        assert_eq!(job.tags, vec!["rust", "postgres"]);
        assert_eq!(job.created_at.year(), 2026);
        let Data::Workatastartup { detail } = &job.raw else {
            panic!("expected workatastartup data");
        };
        assert_eq!(detail.company_name, "VectorShift");
        assert!(detail.has_salary);
        assert!(!detail.has_equity);
        assert_eq!(detail.min_experience, Some(2));
        assert_eq!(detail.us_visa_required, "none");
    }
}
