use anyhow::{Context, Result};
use schemars::{JsonSchema, schema_for};
use serde::Deserialize;
use std::marker::PhantomData;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_TEXT_LEN: usize = 2000;

/// A type that can be extracted from LLM output.
pub trait Extractable: JsonSchema + for<'de> Deserialize<'de> {
    /// Prompt template with `{schema}` and `{text}` placeholders.
    const PROMPT_TEMPLATE: &'static str;

    /// Render the prompt for this extraction target, given the JSON schema and source text.
    fn render_prompt(schema: &str, text: &str) -> String {
        Self::PROMPT_TEMPLATE
            .replace("{schema}", schema)
            .replace("{text}", text)
    }
}

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
pub struct HackerNewsFields {
    #[schemars(description = "true only if the comment is an actual job advertisement")]
    #[serde(default)]
    pub is_job_ad: bool,
    #[schemars(description = "company or organization name")]
    pub company: Option<String>,
    #[schemars(description = "job title or role")]
    pub role: Option<String>,
    #[schemars(description = "location mentioned in the post")]
    pub location: Option<String>,
    #[schemars(
        description = "true if the post explicitly mentions remote, distributed, worldwide, or global work"
    )]
    #[serde(default)]
    pub remote: Option<bool>,
    #[schemars(
        description = "raw compensation snippet (e.g. '$150k-$175k' or 'EUR 80k-100k'), or null"
    )]
    pub budget: Option<String>,
    #[serde(default)]
    #[schemars(
        description = "include 'remote', 'onsite', or 'hybrid' when mentioned, plus tech/stack keywords"
    )]
    pub tags: Vec<String>,
}

impl Extractable for HackerNewsFields {
    const PROMPT_TEMPLATE: &'static str = include_str!("prompts/hackernews_fields.md");
}

/// Generic LLM extractor that calls a local CLI.
///
/// Configure via env:
/// - `JOBSEARCH_LLM_BIN` binary name or path (default `pi`)
/// - `JOBSEARCH_LLM_ARGS` space-separated CLI args appended before the prompt
///   (default: `--print --no-session --no-tools --mode text`)
#[derive(Debug, Clone)]
pub struct LlmExtractor<T: Extractable> {
    bin: String,
    args: Vec<String>,
    _phantom: PhantomData<T>,
}

impl<T: Extractable> LlmExtractor<T> {
    pub async fn extract(&self, text: &str) -> Result<T> {
        let schema = serde_json::to_string_pretty(&schema_for!(T))?;
        let truncated = Self::truncate(text);
        let rendered = T::render_prompt(&schema, &truncated);
        self.run_and_parse::<T>(&rendered).await
    }

    pub fn from_env() -> Self {
        let mut args = std::env::var("JOBSEARCH_LLM_ARGS")
            .map(|s| shell_words::split(&s).unwrap_or_default())
            .unwrap_or_else(|_| {
                vec![
                    "--print".to_string(),
                    "--no-session".to_string(),
                    "--no-tools".to_string(),
                    "--mode".to_string(),
                    "text".to_string(),
                    "--thinking".to_string(),
                    "off".to_string(),
                ]
            });
        if !args.iter().any(|a| a == "--model")
            && let Ok(model) = std::env::var("JOBSEARCH_LLM_MODEL")
        {
            args.extend(["--model".to_string(), model]);
        }
        Self {
            bin: std::env::var("JOBSEARCH_LLM_BIN").unwrap_or_else(|_| "pi".to_string()),
            args,
            _phantom: PhantomData,
        }
    }

    /// Run the LLM with the rendered prompt and deserialize the response into `T`.
    async fn run_and_parse<R>(&self, prompt: &str) -> Result<R>
    where
        R: for<'de> Deserialize<'de>,
    {
        let out = self.run(prompt).await?;
        let out = out.unwrap_or_default();
        if out.is_empty() || out.eq_ignore_ascii_case("none") {
            anyhow::bail!("llm returned empty or NONE response");
        }
        serde_json::from_str(&out).with_context(|| format!("failed to parse LLM JSON: {}", out))
    }

    async fn run(&self, prompt: &str) -> Result<Option<String>> {
        let mut cmd = Command::new(&self.bin);
        cmd.args(&self.args);
        cmd.arg(prompt);

        let output = timeout(DEFAULT_TIMEOUT, cmd.output())
            .await
            .context("llm extractor timed out")?
            .context("failed to run llm extractor")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("llm extractor failed: {}", stderr);
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() || text.eq_ignore_ascii_case("none") {
            Ok(None)
        } else {
            Ok(Some(text))
        }
    }

    fn truncate(text: &str) -> String {
        if text.len() <= MAX_TEXT_LEN {
            text.to_string()
        } else {
            let mut end = MAX_TEXT_LEN;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            text[..end].to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hackernews_prompt_renders_placeholders() {
        let prompt = HackerNewsFields::render_prompt("{}", "world");
        assert!(prompt.contains("JSON schema:\n{}"));
        assert!(prompt.contains("Post:\nworld"));
    }

    #[test]
    fn test_truncate_respects_char_boundaries() {
        let s = "αβγδ".repeat(1000);
        let t = LlmExtractor::<HackerNewsFields>::truncate(&s);
        assert!(t.len() <= MAX_TEXT_LEN);
        assert!(t.is_char_boundary(t.len()));
    }
}
