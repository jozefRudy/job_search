use anyhow::{Result, ensure};
use patterns::Extractable;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize, JsonSchema, Extractable)]
#[extract(
    template = "hackernews_fields.md",
    healthcheck = "hackernews_healthcheck.md",
    healthcheck_context = "Candidate location: Europe"
)]
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
        description = "true if the candidate's own location is allowed to work remotely; a region-restricted remote role that still covers the candidate is true (e.g. 'Remote (EU)' is true for a Europe candidate). false if remote work is limited to a region that excludes the candidate, or is onsite (e.g. 'US only', 'onsite' for a Europe candidate). null only if the remote policy is not mentioned at all"
    )]
    pub remote: Option<bool>,
    #[schemars(description = "raw compensation snippet (e.g. '$150k-$175k' or 'EUR 80k-100k')")]
    pub budget: Option<String>,
    #[schemars(description = "tech/stack keywords")]
    pub tags: Vec<String>,
}

impl ExtractFields {
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
    use patterns::limits::ConcurrencyLimits;
    use patterns::llm::ChatLlm;

    /// Chat client from the env: `JOBSEARCH_LLM_BASE_URL` / `JOBSEARCH_LLM_MODEL`
    /// (defaults: DeepSeek + `deepseek-flash`), key in `DEEPSEEK_API_KEY`.
    fn llm() -> ChatLlm {
        let api_key = std::env::var("DEEPSEEK_API_KEY")
            .expect("DEEPSEEK_API_KEY must be set to the provider key");
        let base_url = std::env::var("JOBSEARCH_LLM_BASE_URL")
            .unwrap_or_else(|_| "https://api.deepseek.com".to_owned());
        let model =
            std::env::var("JOBSEARCH_LLM_MODEL").unwrap_or_else(|_| "deepseek-flash".to_owned());
        ChatLlm::new(base_url, api_key, model, ConcurrencyLimits::default())
    }

    const CONTEXT: &str = "Candidate location: Europe";

    #[tokio::test]
    #[ignore = "requires DEEPSEEK_API_KEY"]
    async fn test_extract_hackernews_job_from_fixture() {
        let text = include_str!("fixtures/hackernews_job.md");
        let fields = llm()
            .extract::<ExtractFields>(text, CONTEXT.to_string())
            .await
            .expect("llm extraction failed");
        assert!(fields.is_job_ad, "expected job ad");
        assert_eq!(fields.company.as_deref(), Some("Stripe"));
        assert_eq!(fields.role.as_deref(), Some("Senior Backend Engineer"));
        assert_eq!(fields.remote, Some(false), "us only");
        assert!(fields.budget.is_some(), "expected budget");
    }

    #[tokio::test]
    #[ignore = "requires DEEPSEEK_API_KEY"]
    async fn test_extract_hackernews_multiple_roles() {
        let text = include_str!("fixtures/hackernews_multiple_roles.md");
        let fields = llm()
            .extract::<ExtractFields>(text, CONTEXT.to_string())
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
