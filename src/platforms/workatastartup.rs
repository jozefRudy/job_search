//! Work at a Startup (Y Combinator) platform client.
//!
//! Backend: public Algolia index `WaaSPublicCompanyJob_created_at_desc_production`
//! (app id `45BWZJ1SGC`). The Algolia secured API key embeds a per-session
//! `userToken` and is only issued to logged-in sessions, so fetching runs inside
//! a workatastartup.com browser tab: JS extracts the full key URL from
//! `performance.getEntriesByType('resource')` and paginates `/1/indexes/*/queries`.

use crate::browser::{BrowserExt, host_of};
use crate::db::Db;
use crate::models::{Job, Platform, WorkAtStartupJobDetail};
use crate::platforms::{FetchState, PlatformClient};
use anyhow::Result;
use async_trait::async_trait;
use chromiumoxide::Page;
use chromiumoxide::browser::Browser;
use serde::Deserialize;

const QUERY_ALGOLIA_JS: &str = include_str!("workatastartup/query_algolia.js");

const ALGOLIA_INDEX: &str = "WaaSPublicCompanyJob_created_at_desc_production";

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
        let _ = url;
        todo!()
    }

    /// Algolia filter expression, e.g.
    /// `(role:eng) AND (remote:only) AND (us_visa_required:none OR us_visa_required:possible)`.
    #[must_use]
    pub fn to_algolia(&self) -> String {
        todo!()
    }
}

/// Algolia hit from the `WaaSPublicCompanyJob` index.
#[derive(Debug, Clone, Deserialize)]
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
    pub min_experience: u32,
    pub created_at: String,
    #[serde(default)]
    pub skills: Vec<String>,
    pub has_salary: bool,
    pub has_equity: bool,
    pub has_interview_process: bool,
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
            anyhow::bail!(
                "Workatastartup requires open workatastartup.com tab in browser (logged in)"
            );
        }
        Ok(())
    }

    /// Evaluate QUERY_ALGOLIA_JS: extract secured key URL from performance
    /// resource entries, POST one page of hits. Bails with login hint when JS
    /// reports missing key.
    async fn fetch_page(
        &self,
        page: &Page,
        filters: &str,
        page_num: u32,
    ) -> Result<AlgoliaPage> {
        let _ = (page, filters, page_num, QUERY_ALGOLIA_JS, ALGOLIA_INDEX);
        todo!()
    }

    /// Map an Algolia hit to a `Job` (external_id = objectID, url = search_path,
    /// remote = hit.remote == "only").
    #[must_use]
    pub fn hit_to_job(hit: &RawWaasHit) -> Job {
        let _ = hit;
        todo!()
    }

    #[must_use]
    fn hit_to_detail(hit: &RawWaasHit) -> WorkAtStartupJobDetail {
        let _ = hit;
        todo!()
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
        let _ = (browser, db, url, pause_ms);
        let _ = Platform::Workatastartup;
        todo!()
    }
}
