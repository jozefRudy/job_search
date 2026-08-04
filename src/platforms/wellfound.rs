//! Wellfound platform client.
//!
//! Backend: Next.js SSR pages at wellfound.com/role/... embed the full Apollo
//! cache as `"apolloState":{"data":{...}}` JSON in the HTML. GraphQL requests
//! are signed (`x-apollo-signature`) and DataDome-protected, so pages are
//! fetched inside a logged-in wellfound.com browser tab via
//! `fetch(url, { credentials: "include" })` and the embedded JSON is
//! extracted — no GraphQL replay.
//!
//! Job entries are `JobListingSearchResult:<id>` cache keys. Company linkage
//! is REVERSED: jobs have no `startup` field — `StartupResult:<id>` entries
//! hold `highlightedJobListings: [{__ref: "JobListingSearchResult:<id>"}]`.
//! Build jobId→startup map from those (one startup can highlight 2+ jobs).
//! Verified nullability (page sample, 37 jobs): remoteConfig null in ~75%
//! (→ remote_kind Option), yearsExperienceMin/Max mostly null (→ Option),
//! compensation/description/liveStartAt/title never null, startup highConcept
//! occasionally null (→ Option).
//! Job URL format: `https://wellfound.com/jobs/<id>-<slug>`.
//! Pagination: `?page=N` (1-based, ~40/page); stop when a page yields no jobs.

use crate::browser::{BrowserExt, host_of};
use crate::db::{Db, UpsertResult};
use crate::models::{Data, Job, Platform, Rating, WellfoundJobDetail};
use crate::platforms::{FetchState, PlatformClient};
use crate::term::CursorGuard;
use anyhow::{Result, bail};
use async_trait::async_trait;
use chromiumoxide::Page;
use chromiumoxide::browser::Browser;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, sleep};

const FETCH_PAGE_JS: &str = include_str!("wellfound/fetch_page.js");
const CONFIG_PLACEHOLDER: &str = "__CONFIG__";

/// Validate `url` (host wellfound.com, path starts with `/role/`) and return
/// it with the `page` query param set. All other params are kept but ignored
/// by the server — only the path filters.
pub fn page_url(url: &str, page: u32) -> Result<String> {
    let _ = (url, page);
    bail!("TODO Phase 3")
}

/// Raw job from Apollo `JobListingSearchResult` with startup fields inlined.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawWellfoundJob {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub description: String,
    /// "$135k – $175k • 0.05% – 0.25%"
    pub compensation: Option<String>,
    /// "full-time"
    pub job_type: String,
    /// Unix seconds; Option per lesson 2 (explicit null not covered by default).
    pub live_start_at: Option<i64>,
    pub location_names: Vec<String>,
    pub remote: bool,
    /// remoteConfig.kind, flattened in JS; null when remoteConfig absent (~75%).
    pub remote_kind: Option<String>,
    pub accepted_remote_location_names: Vec<String>,
    pub years_experience_min: Option<u32>,
    pub years_experience_max: Option<u32>,
    pub primary_role_title: Option<String>,
    // resolved via StartupResult.highlightedJobListings reverse map:
    pub company_name: String,
    pub company_slug: Option<String>,
    /// "SIZE_11_50"
    pub company_size: Option<String>,
    pub company_high_concept: Option<String>,
    pub company_logo_url: Option<String>,
}

/// One SSR page of jobs returned by FETCH_PAGE_JS.
#[derive(Debug, Deserialize)]
pub struct WellfoundPage {
    #[serde(default)]
    pub jobs: Vec<RawWellfoundJob>,
    pub error: Option<String>,
}

pub struct WellfoundScraper;

impl WellfoundScraper {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Bail unless an open tab host contains wellfound.com (user must be
    /// logged in — SSR pages return full data only for sessions).
    async fn ensure_wellfound_tab(&self, browser: &Browser) -> Result<()> {
        let _ = browser;
        bail!("TODO Phase 3")
    }

    /// Evaluate FETCH_PAGE_JS with config {"url": page_url}: JS fetches the
    /// SSR HTML, extracts apolloState by brace matching from
    /// `"apolloState":{"data":{`, collects JobListingSearchResult:* entries,
    /// inlines StartupResult fields via the highlightedJobListings reverse
    /// map, returns {jobs, error}. Bail on error (SSR needs no session token
    /// wait/retry).
    pub async fn fetch_page(&self, page: &Page, page_url: &str) -> Result<WellfoundPage> {
        let _ = (page, page_url);
        bail!("TODO Phase 3")
    }

    /// Map a raw job to a `Job`: external_id = id,
    /// url = https://wellfound.com/jobs/{id}-{slug},
    /// created_at = live_start_at (Utc::now() fallback),
    /// budget = compensation parsed as yearly range (raw string fallback),
    /// company = company_name, remote = remote,
    /// raw = Data::Wellfound { detail }.
    #[must_use]
    pub fn hit_to_job(hit: &RawWellfoundJob) -> Job {
        let _ = hit;
        todo!("Phase 3")
    }

    #[must_use]
    fn hit_to_detail(hit: &RawWellfoundJob) -> WellfoundJobDetail {
        let _ = hit;
        todo!("Phase 3")
    }
}

impl Default for WellfoundScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PlatformClient for WellfoundScraper {
    fn name(&self) -> &'static str {
        "wellfound"
    }

    async fn fetch_with_browser(
        &self,
        browser: &Browser,
        db: &Db,
        url: &str,
        pause_ms: u64,
    ) -> Result<FetchState> {
        let _ = (browser, db, url, pause_ms);
        bail!("TODO Phase 3")
    }
}

#[cfg(test)]
mod tests {
    // Phase 3:
    //   - page_url builds ?page=N / replaces existing page param;
    //     rejects foreign host and non-/role/ path
    //   - hit_to_job maps fields from recorded JSON sample (compensation string,
    //     live_start_at unix, nullable years_experience_*, startup inlined)
    use super::*;
}
