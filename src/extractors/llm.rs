use anyhow::{Context, Result};
use schemars::{JsonSchema, schema_for};
use serde::Deserialize;
use std::marker::PhantomData;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_TEXT_LEN: usize = 4000;

macro_rules! define_prompts {
    ($(($variant:ident, $struct:ident, $path:literal)),* $(,)?) => {
        #[derive(Copy, Clone, Debug)]
        pub enum PromptKind {
            $($variant,)*
        }

        $(
            #[derive(::askama::Template)]
            #[template(path = $path, ext = "md")]
            struct $struct<'a> {
                schema: &'a str,
                text: &'a str,
                prompt_context: &'a str,
            }

            impl<'a> $struct<'a> {
                fn render_prompt(
                    schema: &'a str,
                    text: &'a str,
                    prompt_context: &'a str,
                ) -> ::anyhow::Result<String> {
                    use ::askama::Template;
                    Self { schema, text, prompt_context }
                        .render()
                        .map_err(Into::into)
                }
            }
        )*

        impl PromptKind {
            pub(crate) fn render_prompt(
                self,
                schema: &str,
                text: &str,
                prompt_context: &str,
            ) -> ::anyhow::Result<String> {
                match self {
                    $(Self::$variant => $struct::render_prompt(schema, text, prompt_context),)*
                }
            }
        }
    };
}

define_prompts! {
    (HackerNews, HackerNewsPrompt, "hackernews_fields.md"),
    (RedditRust, RedditRustPrompt, "reddit_rust_fields.md"),
}

/// A type that can be extracted from LLM output.
pub trait Extractable: JsonSchema + for<'de> Deserialize<'de> {
    /// Which prompt template to use for this extraction target.
    const PROMPT: PromptKind;
    /// Sample text used to verify the LLM produces valid structured output.
    const HEALTHCHECK_TEXT: &'static str;
    /// Validate that a healthcheck extraction succeeded.
    fn verify(&self) -> Result<()>;
}

/// Generic LLM extractor that calls a local CLI.
///
/// Configure with a command string via `from_bin` (from `jobsearch.toml` `[llm] bin`).
#[derive(Debug, Clone)]
pub struct LlmExtractor<T: Extractable> {
    bin: String,
    args: Vec<String>,
    prompt_context: String,
    _phantom: PhantomData<T>,
}

impl<T: Extractable> LlmExtractor<T> {
    pub async fn extract(&self, text: &str) -> Result<T> {
        let schema = serde_json::to_string_pretty(&schema_for!(T))?;
        let truncated = truncate(text);
        let rendered = T::PROMPT.render_prompt(&schema, &truncated, &self.prompt_context)?;
        self.run_and_parse::<T>(&rendered).await
    }

    /// Attach dynamic context text rendered into the prompt template.
    #[must_use]
    pub fn with_prompt_context(mut self, context: String) -> Self {
        self.prompt_context = context;
        self
    }

    pub async fn verify(&self) -> Result<()> {
        self.extract(T::HEALTHCHECK_TEXT).await?.verify()
    }

    /// Configure with a command string from `jobsearch.toml` `[llm] bin`.
    #[must_use]
    pub fn from_bin(llm_bin: &str) -> Self {
        let tokens = shell_words::split(llm_bin).unwrap_or_default();
        let (bin, args) = tokens
            .split_first()
            .map(|(h, t)| (h.clone(), t.to_vec()))
            .unwrap_or_default();
        Self {
            bin,
            args,
            prompt_context: String::new(),
            _phantom: PhantomData,
        }
    }

    /// Run the LLM with the rendered prompt and deserialize the response into `T`.
    /// On parse failure, retry once with a repair prompt containing the invalid
    /// output and the serde error.
    async fn run_and_parse<R>(&self, prompt: &str) -> Result<R>
    where
        R: for<'de> Deserialize<'de>,
    {
        match self.try_parse(prompt).await {
            Ok(parsed) => Ok(parsed),
            Err(failure) => {
                let repair = build_repair_prompt(prompt, &failure.output, &failure.error);
                self.try_parse(&repair).await.map_err(|retry| {
                    retry.error.context(format!(
                        "llm parse failed after one retry; first error: {}; first output: {}",
                        failure.error, failure.output
                    ))
                })
            }
        }
    }

    async fn try_parse<R>(&self, prompt: &str) -> Result<R, ParseFailure>
    where
        R: for<'de> Deserialize<'de>,
    {
        let out = self.run(prompt).await.map_err(|e| ParseFailure {
            error: e,
            output: String::new(),
        })?;
        let out = out.unwrap_or_default();
        if out.is_empty() || out.eq_ignore_ascii_case("none") {
            return Err(ParseFailure {
                error: anyhow::anyhow!("llm returned empty or NONE response"),
                output: out,
            });
        }
        let stripped = strip_json_fences(&out);
        serde_json::from_str(&stripped).map_err(|e| ParseFailure {
            error: anyhow::Error::new(e).context(format!("failed to parse LLM JSON: {stripped}")),
            output: out,
        })
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
            anyhow::bail!("llm extractor failed: {stderr}");
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() || text.eq_ignore_ascii_case("none") {
            Ok(None)
        } else {
            Ok(Some(text))
        }
    }
}

struct ParseFailure {
    error: anyhow::Error,
    output: String,
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

const REPAIR_PROMPT: &str = include_str!("prompts/repair.md");

fn build_repair_prompt(
    original_prompt: &str,
    previous_output: &str,
    error: &anyhow::Error,
) -> String {
    REPAIR_PROMPT
        .replace("{original_prompt}", original_prompt)
        .replace("{previous_output}", &truncate(previous_output))
        .replace("{error}", &format!("{error:#}"))
}

fn strip_json_fences(text: &str) -> String {
    let trimmed = text.trim();
    if let Some(body) = trimmed
        .strip_prefix("```json")
        .and_then(|s| s.trim_end().strip_suffix("```"))
    {
        body.trim().to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hackernews_prompt_renders_placeholders() {
        let prompt = PromptKind::HackerNews
            .render_prompt("{}", "world", "Candidate location: Europe")
            .expect("missing template variables");
        assert!(prompt.contains("JSON schema:\n{}"));
        assert!(prompt.contains("Post:\nworld"));
        assert!(prompt.contains("Candidate location: Europe"));
    }

    #[test]
    fn test_truncate_respects_char_boundaries() {
        let s = "αβγδ".repeat(1000);
        let t = truncate(&s);
        assert!(t.len() <= MAX_TEXT_LEN);
        assert!(t.is_char_boundary(t.len()));
    }

    #[test]
    fn test_build_repair_prompt_contains_context() {
        let err = anyhow::anyhow!("expected value at line 1 column 2");
        let prompt = build_repair_prompt("ORIGINAL PROMPT", "not json at all", &err);
        assert!(prompt.contains("ORIGINAL PROMPT"));
        assert!(prompt.contains("not json at all"));
        assert!(prompt.contains("expected value at line 1 column 2"));
    }

    #[test]
    fn test_build_repair_prompt_truncates_long_output() {
        let err = anyhow::anyhow!("boom");
        let long = "x".repeat(MAX_TEXT_LEN * 2);
        let prompt = build_repair_prompt("p", &long, &err);
        assert!(!prompt.contains(&"x".repeat(MAX_TEXT_LEN + 1)));
    }

    #[test]
    fn test_strip_json_fences_removes_fences() {
        let raw = "```json\n{\"is_job_ad\": true}\n```";
        assert_eq!(strip_json_fences(raw), "{\"is_job_ad\": true}");
    }

    #[test]
    fn test_strip_json_fences_leaves_plain_json() {
        let raw = "{\"is_job_ad\": true}";
        assert_eq!(strip_json_fences(raw), "{\"is_job_ad\": true}");
    }
}
