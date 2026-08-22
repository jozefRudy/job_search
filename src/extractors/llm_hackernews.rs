use crate::extractors::llm::{Extractable, PromptKind};
use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
pub struct ExtractFields {
    #[schemars(description = "true only if the comment is an actual job advertisement")]
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
        description = "true if fully remote from the candidate's location, false if location-restricted (e.g. 'US only', 'onsite'); null only if remote policy is not mentioned at all"
    )]
    pub remote: Option<bool>,
    #[schemars(description = "raw compensation snippet (e.g. '$150k-$175k' or 'EUR 80k-100k')")]
    pub budget: Option<String>,
    #[schemars(description = "tech/stack keywords")]
    pub tags: Vec<String>,
}

impl Extractable for ExtractFields {
    const PROMPT: PromptKind = PromptKind::HackerNews;
    const HEALTHCHECK_TEXT: &'static str = include_str!("llm/fixtures/hackernews_healthcheck.md");

    fn verify(&self) -> Result<()> {
        ensure!(
            self.is_job_ad,
            "healthcheck text must be classified as a job ad"
        );
        let company = self.company.as_deref().unwrap_or_default();
        ensure!(
            company.to_lowercase().contains("acme"),
            "healthcheck company extraction failed: {company:?}"
        );
        let role = self.role.as_deref().unwrap_or_default();
        ensure!(
            role.to_lowercase().contains("rust"),
            "healthcheck role extraction failed: {role:?}"
        );
        ensure!(
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
    use crate::extractors::llm::LlmExtractor;

    fn llm_bin() -> String {
        std::env::var("JOBSEARCH_LLM_BIN")
            .expect("JOBSEARCH_LLM_BIN must be set to an LLM CLI command")
    }

    #[tokio::test]
    #[ignore = "requires JOBSEARCH_LLM_BIN set to an LLM CLI command"]
    async fn test_extract_hackernews_job_from_fixture() {
        let text = include_str!("llm/fixtures/hackernews_job.md");
        let fields = LlmExtractor::<ExtractFields>::from_bin(&llm_bin())
            .with_prompt_context("Candidate location: Europe".to_string())
            .extract(text)
            .await
            .expect("llm extraction failed");
        assert!(fields.is_job_ad, "expected job ad");
        assert_eq!(fields.company.as_deref(), Some("Stripe"));
        assert_eq!(fields.role.as_deref(), Some("Senior Backend Engineer"));
        assert_eq!(fields.remote, Some(false), "us only");
        assert!(fields.budget.is_some(), "expected budget");
    }

    #[tokio::test]
    #[ignore = "requires JOBSEARCH_LLM_BIN set to an LLM CLI command"]
    async fn test_extract_hackernews_multiple_roles() {
        let text = include_str!("llm/fixtures/hackernews_multiple_roles.md");
        let fields = LlmExtractor::<ExtractFields>::from_bin(&llm_bin())
            .with_prompt_context("Candidate location: Europe".to_string())
            .extract(text)
            .await
            .expect("llm extraction failed");
        assert!(fields.is_job_ad, "expected job ad");
        assert_eq!(fields.company.as_deref(), Some("Close"));
        let role = fields.role.as_deref().unwrap_or_default();
        assert!(
            role.to_lowercase().contains("backend"),
            "expected backend in joined roles, got {role:?}"
        );
        assert!(
            role.chars().filter(|c| *c == '+').count() == 3,
            "expected 4 roles joined, got {role:?}"
        );
        assert_eq!(fields.remote, Some(false), "expected not remote (us only)");
    }
}
