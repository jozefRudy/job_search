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
        query: &str,
        pause_ms: u64,
    ) -> Result<FetchState>;

    // TODO(phase1): Update `fetch_with_browser` signature: `query: &str` -> `url: &str`.
    // Browser-based providers treat it as a URL; HN ignores it and uses `self.location`.
    async fn fetch_with_manager(
        &self,
        manager: &BrowserManager,
        db: &Db,
        query: &str,
        pause_ms: u64,
    ) -> Result<FetchState> {
        let browser = manager.browser().await?;
        self.fetch_with_browser(&browser, db, query, pause_ms).await
    }

    async fn sync_applications(
        &self,
        _browser: &Browser,
        _db: &Db,
        _pause_ms: u64,
        _limit: Option<usize>,
    ) -> Result<FetchState> {
        // TODO(phase1): Remove `sync_applications` from the trait (feature removed).
        Err(anyhow::anyhow!(
            "sync_applications not implemented for {}",
            self.name()
        ))
    }
}

pub mod fetch_state;
pub use fetch_state::FetchState;

pub mod efinancialcareers;
pub mod hackernews;
pub mod linkedin;
pub mod nofluffjobs;
pub mod upwork;
