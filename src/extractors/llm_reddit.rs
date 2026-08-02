//! Reddit LLM extraction fields.
//!
//! Two structs, SAME field shape, different prompts:
//! - JobbitFields: r/jobbit posts — semi-structured titles
//!   (role @ company, location, salary often in title alone).
//!   is_job_ad required: "Hiring" flair still contains junk (meta posts,
//!   challenges, seeker referrals under "Hiring - Open" flair); [HIRING]
//!   title prefix not reliable.
//! - MegathreadFields: r/rust "Who's Hiring" comments — prose; prompt must
//!   reject job-SEEKER presentations (single thread for seekers + offerers;
//!   seekers reply under designated top-level comment) and mod/meta/deleted
//!   comments; extract only job offers.
//! Extractable::PROMPT is a per-struct const, hence two structs (no shared
//! abstraction — YAGNI).

use crate::extractors::llm::{Extractable, PromptKind};
use anyhow::Result;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
pub struct JobbitFields {
    #[schemars(description = "true only if the post is an actual job advertisement; false for meta posts, challenges, and job-seeker posts")]
    pub is_job_ad: bool,
    #[schemars(description = "company or organization name")]
    pub company: Option<String>,
    #[schemars(description = "job title or role; if multiple listed, join them with ' + '")]
    pub role: Option<String>,
    #[schemars(
        description = "location mentioned in the post, if multiple listed, join them with ' + '"
    )]
    pub location: Option<String>,
    #[schemars(
        description = "true only if fully remote work is allowed from the candidate's location"
    )]
    pub remote: Option<bool>,
    #[schemars(description = "raw compensation snippet (e.g. '$150k-$175k' or 'EUR 80k-100k')")]
    pub budget: Option<String>,
    #[schemars(description = "tech/stack keywords")]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
pub struct MegathreadFields {
    #[schemars(description = "true only if the comment is an actual job offer; false for job-seeker presentations, mod/meta comments, and non-employment posts like grant funding or open calls")]
    pub is_job_ad: bool,
    #[schemars(description = "company or organization name")]
    pub company: Option<String>,
    #[schemars(description = "job title or role; if multiple listed, join them with ' + '")]
    pub role: Option<String>,
    #[schemars(
        description = "location mentioned in the comment, if multiple listed, join them with ' + '"
    )]
    pub location: Option<String>,
    #[schemars(
        description = "true only if fully remote work is allowed from the candidate's location"
    )]
    pub remote: Option<bool>,
    #[schemars(description = "raw compensation snippet (e.g. '$150k-$175k' or 'EUR 80k-100k')")]
    pub budget: Option<String>,
    #[schemars(description = "tech/stack keywords")]
    pub tags: Vec<String>,
}

impl Extractable for JobbitFields {
    const PROMPT: PromptKind = PromptKind::RedditJobbit;
    const HEALTHCHECK_TEXT: &'static str =
        include_str!("llm/fixtures/reddit_jobbit_healthcheck.md");

    fn verify(&self) -> Result<()> {
        todo!("phase3: is_job_ad, company contains \"acme\", role contains \"rust\", remote Some(true)")
    }
}

impl Extractable for MegathreadFields {
    const PROMPT: PromptKind = PromptKind::RedditMegathread;
    const HEALTHCHECK_TEXT: &'static str =
        include_str!("llm/fixtures/reddit_megathread_healthcheck.md");

    fn verify(&self) -> Result<()> {
        todo!("phase3: same expectations as JobbitFields::verify")
    }
}

// TODO(phase3): ignored integration tests mirroring llm_hackernews.rs tests
//   (#[ignore = "requires JOBSEARCH_LLM_CLI set to an LLM CLI command"];
//    fixture via include_str!("llm/fixtures/reddit_*.md"), .expect() not unwrap):
//   test_extract_jobbit_from_fixture — job ad: is_job_ad, company, role, remote,
//     budget parsed from [HIRING] title alone
//   test_extract_jobbit_rejects_non_job — fixture: meta/"I got the job!"-style post
//     => is_job_ad == false
//   test_extract_megathread_from_fixture — job ad from prose comment
//   test_extract_megathread_rejects_seeker — job-seeker comment fixture
//     => is_job_ad == false
//   test_extract_megathread_rejects_grant — grant-funding/open-call fixture
//     => is_job_ad == false (not employment)
