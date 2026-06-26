use anyhow::{Context, Result};
use schemars::{JsonSchema, schema_for};
use serde::Deserialize;
use std::marker::PhantomData;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

pub const DEFAULT_LLM_CLI: &str = "pi --print --no-session --no-tools --no-extensions --mode text --thinking off --model deepseek/deepseek-v4-flash";

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
            }

            impl<'a> $struct<'a> {
                fn render_prompt(schema: &'a str, text: &'a str) -> ::anyhow::Result<String> {
                    use ::askama::Template;
                    Self { schema, text }.render().map_err(Into::into)
                }
            }
        )*

        impl PromptKind {
            pub(crate) fn render_prompt(self, schema: &str, text: &str) -> ::anyhow::Result<String> {
                match self {
                    $(Self::$variant => $struct::render_prompt(schema, text),)*
                }
            }
        }
    };
}

define_prompts! {
    (HackerNews, HackerNewsPrompt, "hackernews_fields.md"),
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
/// Configure by passing a command string to `from_cli`. When omitted,
/// [`DEFAULT_LLM_CLI`] is used.
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
        let rendered = T::PROMPT.render_prompt(&schema, &truncated)?;
        self.run_and_parse::<T>(&rendered).await
    }

    pub async fn verify(&self) -> Result<()> {
        self.extract(T::HEALTHCHECK_TEXT).await?.verify()
    }

    #[must_use]
    pub fn from_cli(llm_cli: Option<String>) -> Self {
        let command = llm_cli
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_LLM_CLI.to_string());
        let tokens = shell_words::split(&command).unwrap_or_default();
        let (bin, args) = tokens
            .split_first()
            .map(|(h, t)| (h.clone(), t.to_vec()))
            .unwrap_or_default();
        Self {
            bin,
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
        let out = strip_json_fences(&out);
        serde_json::from_str(&out).with_context(|| format!("failed to parse LLM JSON: {out}"))
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
    use crate::extractors::llm_hackernews;

    #[test]
    fn test_hackernews_prompt_renders_placeholders() {
        let prompt = PromptKind::HackerNews
            .render_prompt("{}", "world")
            .expect("missing template variables");
        assert!(prompt.contains("JSON schema:\n{}"));
        assert!(prompt.contains("Post:\nworld"));
    }

    #[test]
    fn test_truncate_respects_char_boundaries() {
        let s = "αβγδ".repeat(1000);
        let t = LlmExtractor::<llm_hackernews::ExtractFields>::truncate(&s);
        assert!(t.len() <= MAX_TEXT_LEN);
        assert!(t.is_char_boundary(t.len()));
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
