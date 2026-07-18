use crate::browser::BrowserManager;
use crate::db::Db;
use anyhow::Result;
use chromiumoxide::browser::Browser;

#[async_trait::async_trait]
pub trait PlatformClient: Send + Sync {
    fn name(&self) -> &'static str;

    async fn fetch_with_browser(
        &self,
        browser: &Browser,
        db: &Db,
        url: &str,
        pause_ms: u64,
    ) -> Result<FetchState>;

    async fn fetch_with_manager(
        &self,
        manager: &BrowserManager,
        db: &Db,
        url: &str,
        pause_ms: u64,
    ) -> Result<FetchState> {
        let browser = manager.browser().await?;
        self.fetch_with_browser(&browser, db, url, pause_ms).await
    }
}

pub mod fetch_state;
pub use fetch_state::FetchState;

pub mod efinancialcareers;
pub mod hackernews;
pub mod linkedin;
pub mod nofluffjobs;
pub mod upwork;
