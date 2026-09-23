//! Reddit LLM extraction fields.
//!
//! RustFields: r/rust "Who's Hiring" comments — prose; prompt must
//! reject job-SEEKER presentations (single thread for seekers + offerers;
//! seekers reply under designated top-level comment), mod/meta/deleted
//! comments, and non-employment content (grant funding, open calls);
//! extract only job offers.

use anyhow::Result;
use patterns::Extractable;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize, JsonSchema, Extractable)]
#[extract(
    template = "reddit_rust_fields.md",
    healthcheck = "reddit_rust_healthcheck.md"
)]
pub struct RustFields {
    #[schemars(
        description = "true only if the comment is an actual job offer; false for job-seeker presentations, mod/meta comments, and non-employment posts like grant funding or open calls"
    )]
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

impl RustFields {
    fn verify(&self) -> Result<()> {
        anyhow::ensure!(
            self.is_job_ad,
            "healthcheck text must be classified as a job ad"
        );
        let company = self.company.as_deref().unwrap_or_default();
        anyhow::ensure!(
            company.to_lowercase().contains("acme"),
            "healthcheck company extraction failed: {company:?}"
        );
        let role = self.role.as_deref().unwrap_or_default();
        anyhow::ensure!(
            role.to_lowercase().contains("rust"),
            "healthcheck role extraction failed: {role:?}"
        );
        anyhow::ensure!(
            self.remote == Some(true),
            "healthcheck remote extraction failed: {:?}",
            self.remote
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use patterns::llm_cli::{ConcurrencyLimits, SharedLlm};

    /// JOBSEARCH_LLM_BIN is a command string (bin + args); split once here.
    fn llm() -> SharedLlm {
        let cmd = std::env::var("JOBSEARCH_LLM_BIN")
            .expect("JOBSEARCH_LLM_BIN must be set to an LLM CLI command");
        let mut parts = cmd.split_whitespace().map(String::from);
        let bin = parts.next().expect("JOBSEARCH_LLM_BIN must have a bin");
        SharedLlm::new(
            bin,
            parts.collect::<Vec<String>>(),
            ConcurrencyLimits::default(),
        )
    }

    const CONTEXT: &str = "Candidate location: Europe";

    #[tokio::test]
    #[ignore = "requires JOBSEARCH_LLM_BIN set to an LLM CLI command"]
    async fn test_extract_rust_from_fixture() {
        let text = include_str!("fixtures/reddit_rust_job.md");
        let fields = llm()
            .extract::<RustFields>(text, CONTEXT.to_string())
            .await
            .expect("llm extraction failed");
        assert!(fields.is_job_ad, "expected job ad");
        assert_eq!(fields.company.as_deref(), Some("PgDog"));
        assert!(fields.budget.is_some(), "expected budget");
    }

    #[tokio::test]
    #[ignore = "requires JOBSEARCH_LLM_BIN set to an LLM CLI command"]
    async fn test_extract_rust_rejects_seeker() {
        let text = include_str!("fixtures/reddit_rust_seeker.md");
        let fields = llm()
            .extract::<RustFields>(text, CONTEXT.to_string())
            .await
            .expect("llm extraction failed");
        assert!(!fields.is_job_ad, "job-seeker comment must not be a job ad");
    }

    #[tokio::test]
    #[ignore = "requires JOBSEARCH_LLM_BIN set to an LLM CLI command"]
    async fn test_extract_rust_rejects_grant() {
        let text = include_str!("fixtures/reddit_rust_grant.md");
        let fields = llm()
            .extract::<RustFields>(text, CONTEXT.to_string())
            .await
            .expect("llm extraction failed");
        assert!(
            !fields.is_job_ad,
            "grant funding / open call must not be a job ad"
        );
    }
}
