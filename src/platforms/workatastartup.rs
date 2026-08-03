//! Work at a Startup (Y Combinator) platform client.
//!
//! Backend: public Algolia index `WaaSPublicCompanyJob_created_at_desc_production`
//! (app id `45BWZJ1SGC`). The Algolia secured API key embeds a per-session
//! `userToken` and is only issued to logged-in sessions, so fetching runs inside
//! a workatastartup.com browser tab: JS extracts the full key URL from
//! `performance.getEntriesByType('resource')` and paginates `/1/indexes/*/queries`.

// TODO(phase2): const QUERY_ALGOLIA_JS: &str = include_str!("workatastartup/query_algolia.js");
//   JS: find algolia request URL in performance resource entries (contains
//   "algolia.net/1/indexes" and "x-algolia-api-key"), then POST
//   `{requests:[{indexName, params}]}` with params built from passed filters +
//   `hitsPerPage=100&page=N`. Args: (filters: string, page: number).
//   Returns {hits: [...], nbPages} or {error} when key absent (not logged in).

// TODO(phase2): struct WaasFilters { role, remote, us_visa_not_required, job_type, sort }
//   - `WaasFilters::from_url(url: &str) -> Result<WaasFilters>` — parse
//     workatastartup.com/companies query params (role=eng, remote=only,
//     usVisaNotRequired=true, jobType=any, sortBy=created_desc).
//     Bail on non-workatastartup.com host.
//   - `to_algolia(&self) -> String` — e.g.
//     `(role:eng) AND (remote:only) AND (us_visa_required:none OR us_visa_required:possible)`

// TODO(phase2): struct RawWaasHit (Deserialize) — Algolia hit fields:
//   objectID, title, description, job_type, remote, role, eng_type: Vec<String>,
//   min_experience: u32, created_at: String, skills: Vec<String>,
//   has_salary: bool, has_equity: bool, has_interview_process: bool,
//   us_visa_required: String, company_id: u64, company_name: String,
//   company_website: Option<String>, company_description: Option<String>,
//   company_parent_sector: Option<String>, company_team_size: Option<u32>,
//   company_waas_stage: Option<String>, search_path: String

// TODO(phase2): pub struct WorkatastartupScraper
//   - `new() -> Self`
//   - `ensure_waas_tab(&self, browser: &Browser) -> Result<()>` — bail unless an
//     open tab host contains workatastartup.com (user must be logged in)
//   - `fetch_page(&self, page: &Page, filters: &str, page_num: u32) -> Result<AlgoliaPage>`
//     evaluating QUERY_ALGOLIA_JS; bail with login hint when JS returns error
//   - hit → Job mapping: external_id = objectID, url = search_path,
//     company = company_name, remote = (remote == "only"),
//     raw = Data::Workatastartup { detail: WorkAtStartupJobDetail }

// TODO(phase2): impl PlatformClient for WorkatastartupScraper
//   - `name() -> "workatastartup"`
//   - `fetch_with_browser(browser, db, url, pause_ms) -> Result<FetchState>` —
//     parse filters from url, open tab on workatastartup.com/companies (url),
//     paginate Algolia until nbPages exhausted or all hits already in db
//     (early-stop like upwork detail_ttl), upsert each hit as Job
