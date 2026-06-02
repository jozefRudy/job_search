use crate::browser::BrowserManager;
use crate::models::{Job, Reaction};
use anyhow::Result;
use chromiumoxide::browser::Browser;

#[async_trait::async_trait]
pub trait PlatformClient: Send + Sync {
    fn name(&self) -> &'static str;

    async fn fetch_with_browser(&self, browser: &Browser, query: &str) -> Result<Vec<Job>>;

    async fn fetch_with_manager(&self, manager: &BrowserManager, query: &str) -> Result<Vec<Job>> {
        let browser = manager.ensure().await?;
        self.fetch_with_browser(&browser, query).await
    }

    async fn react(&self, _job: &Job, _action: Reaction) -> Result<()> {
        Err(anyhow::anyhow!("react not implemented for {}", self.name()))
    }
}

pub mod nofluffjobs;
pub mod upwork;
