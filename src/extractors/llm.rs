patterns::define_prompts! {
    (HackerNews, "hackernews_fields.md"),
    (RedditRust, "reddit_rust_fields.md"),
}

pub use patterns::llm_cli::{Extractable, LlmExtractor};

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
}
