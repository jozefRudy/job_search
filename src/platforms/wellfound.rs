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
use crate::language::LanguageService;
use crate::models::{Data, Job, Platform, Rating, WellfoundJobDetail, classify_language};
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

/// Listings are ordered by liveStartAt desc; older than this is not worth
/// scraping. Stop when the NEWEST job on a page exceeds the cutoff — immune
/// to intra-page reordering, everything after is older still.
const MAX_JOB_AGE_DAYS: i64 = 31;

/// Validate `url` (host wellfound.com, path starts with `/role/`) and return
/// it with the `page` query param set. All other params are kept but ignored
/// by the server — only the path filters.
pub fn page_url(url: &str, page: u32) -> Result<String> {
    let mut parsed = url::Url::parse(url)?;
    let host = parsed.host_str().unwrap_or_default();
    if host != "wellfound.com" && !host.ends_with(".wellfound.com") {
        bail!("Wellfound URL must be on wellfound.com");
    }
    if !parsed.path().starts_with("/role/") {
        bail!("Wellfound URL path must start with /role/");
    }
    let kept: Vec<(String, String)> = parsed
        .query_pairs()
        .filter(|(k, _)| k != "page")
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    parsed
        .query_pairs_mut()
        .clear()
        .extend_pairs(kept)
        .append_pair("page", &page.to_string());
    Ok(parsed.to_string())
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
    /// Unix seconds; always present in practice (0/N null verified) —
    /// required, fail fast if Wellfound starts omitting it.
    pub live_start_at: i64,
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
    /// True when the server redirected an out-of-range page back to page 1 —
    /// stop pagination (jobs alone going empty never happens otherwise).
    #[serde(default)]
    pub end: bool,
    pub error: Option<String>,
}

pub struct WellfoundScraper {
    lang: LanguageService,
}

impl WellfoundScraper {
    #[must_use]
    pub fn new(lang: LanguageService) -> Self {
        Self { lang }
    }

    /// Bail unless an open tab host contains wellfound.com (user must be
    /// logged in — SSR pages return full data only for sessions).
    async fn ensure_wellfound_tab(&self, browser: &Browser) -> Result<()> {
        let page_hosts: Vec<_> = browser
            .get_page_urls()
            .await?
            .into_iter()
            .filter_map(|u| host_of(&u))
            .collect();
        if !page_hosts.iter().any(|h| h.contains("wellfound.com")) {
            bail!("Wellfound requires open wellfound.com tab in browser (logged in)");
        }
        Ok(())
    }

    /// Evaluate FETCH_PAGE_JS with config {"url", "page"}: JS fetches the
    /// SSR HTML, extracts apolloState by brace matching from
    /// `"apolloState":{"data":{`, collects JobListingSearchResult:* entries,
    /// inlines StartupResult fields via the highlightedJobListings reverse
    /// map, returns {jobs, error}. Bail on error (SSR needs no session token
    /// wait/retry).
    pub async fn fetch_page(
        &self,
        page: &Page,
        page_url: &str,
        page_num: u32,
    ) -> Result<WellfoundPage> {
        let config = serde_json::json!({ "url": page_url, "page": page_num });
        let js = FETCH_PAGE_JS.replace(CONFIG_PLACEHOLDER, &config.to_string());
        let result: WellfoundPage = page.evaluate(js.as_str()).await?.into_value()?;
        if let Some(e) = result.error {
            bail!("wellfound fetch failed: {e}");
        }
        Ok(result)
    }

    /// Map a raw job to a `Job`: external_id = id,
    /// url = https://wellfound.com/jobs/{id}-{slug},
    /// created_at = live_start_at (Utc::now() fallback),
    /// budget = compensation parsed as yearly range (raw string fallback),
    /// company = company_name, remote = remote,
    /// raw = Data::Wellfound { detail }.
    #[must_use]
    pub fn hit_to_job(hit: &RawWellfoundJob) -> Job {
        let created_at = DateTime::from_timestamp(hit.live_start_at, 0).unwrap_or_else(Utc::now);
        Job {
            id: 0,
            platform: Platform::Wellfound,
            external_id: hit.id.clone(),
            title: hit.title.clone(),
            url: format!("https://wellfound.com/jobs/{}-{}", hit.id, hit.slug),
            budget: hit
                .compensation
                .as_ref()
                .and_then(|c| crate::models::Budget::parse(c, Some("year")))
                .map(|b| b.to_string()),
            tags: Vec::new(),
            raw: Data::Wellfound {
                detail: Self::hit_to_detail(hit),
            },
            company: Some(hit.company_name.clone()).filter(|c| !c.is_empty()),
            created_at,
            updated_at: Utc::now(),
            note: None,
            rating: Rating::Neutral,
            applied_at: None,
            remote: hit.remote,
        }
    }

    #[must_use]
    fn hit_to_detail(hit: &RawWellfoundJob) -> WellfoundJobDetail {
        WellfoundJobDetail {
            description: hit.description.clone(),
            compensation: hit.compensation.clone(),
            job_type: hit.job_type.clone(),
            location_names: hit.location_names.clone(),
            remote: hit.remote,
            remote_kind: hit.remote_kind.clone(),
            accepted_remote_location_names: hit.accepted_remote_location_names.clone(),
            years_experience_min: hit.years_experience_min,
            years_experience_max: hit.years_experience_max,
            primary_role_title: hit.primary_role_title.clone(),
            company_name: hit.company_name.clone(),
            company_slug: hit.company_slug.clone(),
            company_size: hit.company_size.clone(),
            company_high_concept: hit.company_high_concept.clone(),
            company_logo_url: hit.company_logo_url.clone(),
            posted_at: DateTime::from_timestamp(hit.live_start_at, 0).unwrap_or_else(Utc::now),
        }
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
        self.ensure_wellfound_tab(browser).await?;
        let page = browser.new_tab(url).await?;
        let mut state = FetchState::new();
        let _guard = CursorGuard::new();

        let mut page_num = 1u32;
        loop {
            let result = self
                .fetch_page(&page, &page_url(url, page_num)?, page_num)
                .await?;
            if result.end || result.jobs.is_empty() {
                break;
            }

            for hit in &result.jobs {
                // Age-gate first: cheap in-memory filter. Stored jobs aging
                // past the cutoff are unaffected — they stay in the DB, just
                // count as skipped instead of existing on re-runs.
                if Utc::now().timestamp() - hit.live_start_at > MAX_JOB_AGE_DAYS * 24 * 3600 {
                    state.inc_skipped();
                    continue;
                }
                if db
                    .job_updated_at(&Platform::Wellfound, &hit.id)
                    .await?
                    .is_some()
                {
                    state.inc_existing();
                } else if db.is_rejected(&Platform::Wellfound, &hit.id).await? {
                    // Rejected in a previous run (non_english) — auto-skip
                    // like nofluffjobs: no re-classify, no re-mark.
                    state.inc_skipped();
                } else {
                    let job = Self::hit_to_job(hit);
                    if !classify_language(&self.lang, &job).await? {
                        db.mark_rejected(&Platform::Wellfound, &hit.id, "non_english")
                            .await?;
                        state.inc_skipped();
                        continue;
                    }
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
            sleep(Duration::from_millis(pause_ms)).await;
        }
        page.close().await.ok();
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_url_appends_page() {
        let u = page_url("https://wellfound.com/role/r/software-engineer", 3).expect("valid url");
        assert_eq!(u, "https://wellfound.com/role/r/software-engineer?page=3");
    }

    #[test]
    fn test_page_url_allows_subdomain() {
        let u = page_url("https://www.wellfound.com/role/r/software-engineer", 1)
            .expect("www subdomain valid");
        assert!(u.starts_with("https://www.wellfound.com/"));
    }

    #[test]
    fn test_page_url_replaces_existing_page() {
        let u = page_url("https://wellfound.com/role/r/software-engineer?page=1", 2)
            .expect("valid url");
        assert_eq!(u, "https://wellfound.com/role/r/software-engineer?page=2");
    }

    #[test]
    fn test_page_url_rejects_foreign_host() {
        assert!(page_url("https://example.com/role/r/software-engineer", 1).is_err());
        assert!(page_url("https://evilwellfound.com/role/r/x", 1).is_err());
    }

    #[test]
    fn test_page_url_rejects_non_role_path() {
        assert!(page_url("https://wellfound.com/jobs", 1).is_err());
    }

    // Recorded from a live /role/r/software-engineer page (Keeper listing).
    const SAMPLE_HIT: &str = r#"{
        "id": "3317746",
        "title": "Software Engineer",
        "slug": "software-engineer",
        "description": "Mission: Keeper is an AI-powered human-in-the-loop service.",
        "compensation": "$135k – $175k • 0.05% – 0.25%",
        "job_type": "full-time",
        "live_start_at": 1785272337,
        "location_names": ["San Francisco"],
        "remote": true,
        "remote_kind": "ONSITE_OR_REMOTE",
        "accepted_remote_location_names": ["United States"],
        "years_experience_min": 0,
        "years_experience_max": null,
        "primary_role_title": "Software Engineer",
        "company_name": "Keeper",
        "company_slug": "keeper-tax",
        "company_size": "SIZE_11_50",
        "company_high_concept": "File your complex taxes confidently",
        "company_logo_url": "https://photos.wellfound.com/startups/i/6809417-40854095dfa36880444ad789aa5d00c1-medium_jpg.jpg?buster=1666129847"
    }"#;

    #[test]
    fn test_hit_to_job_maps_fields() {
        let hit: RawWellfoundJob =
            serde_json::from_str(SAMPLE_HIT).expect("sample hit deserializes");
        let job = WellfoundScraper::hit_to_job(&hit);
        assert_eq!(job.external_id, "3317746");
        assert_eq!(job.platform, Platform::Wellfound);
        assert_eq!(
            job.url,
            "https://wellfound.com/jobs/3317746-software-engineer"
        );
        assert_eq!(job.company.as_deref(), Some("Keeper"));
        assert!(job.remote);
        assert_eq!(job.created_at.timestamp(), 1_785_272_337);
        assert_eq!(job.budget.as_deref(), Some("135000 - 175000 USD/year"));
        let Data::Wellfound { detail } = &job.raw else {
            panic!("expected wellfound data");
        };
        assert_eq!(detail.company_name, "Keeper");
        assert_eq!(detail.job_type, "full-time");
        assert_eq!(detail.remote_kind.as_deref(), Some("ONSITE_OR_REMOTE"));
        assert_eq!(detail.years_experience_min, Some(0));
        assert_eq!(detail.years_experience_max, None);
        assert_eq!(detail.company_size.as_deref(), Some("SIZE_11_50"));
        assert!(detail.company_logo_url.is_some());
    }

    #[test]
    fn test_hit_nullable_fields_deserialize() {
        let json = r#"{
            "id": "1", "title": "t", "slug": "s", "description": "d",
            "compensation": null, "job_type": "contract", "live_start_at": 1700000000,
            "location_names": [], "remote": false, "remote_kind": null,
            "accepted_remote_location_names": [],
            "years_experience_min": null, "years_experience_max": null,
            "primary_role_title": null, "company_name": "c",
            "company_slug": null, "company_size": null,
            "company_high_concept": null, "company_logo_url": null
        }"#;
        let hit: RawWellfoundJob = serde_json::from_str(json).expect("all-null hit deserializes");
        let job = WellfoundScraper::hit_to_job(&hit);
        assert_eq!(job.budget, None);
        assert!(!job.remote);
    }
}
