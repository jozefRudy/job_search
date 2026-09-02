patterns::define_prompts! {
    (HackerNewsPrompt, "hackernews_fields.md"),
    (RedditRustPrompt, "reddit_rust_fields.md"),
}

pub mod budget;
pub mod llm_hackernews;
pub mod llm_reddit;
