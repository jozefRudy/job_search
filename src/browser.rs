use anyhow::Result;
use chromiumoxide::browser::Browser;
use chromiumoxide::cdp::browser_protocol::target::{
    CloseTargetParams, CreateTargetParams, GetTargetsParams,
};
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::Mutex;

const CDP_URL: &str = "http://localhost:9222";
const BROWSER_APP: &str = "Brave Browser";

pub fn host_of(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()?
        .host_str()
        .map(|h| h.strip_prefix("www.").unwrap_or(h).to_lowercase())
}

#[allow(async_fn_in_trait)]
pub trait BrowserExt {
    async fn new_blank_tab(&self) -> Result<chromiumoxide::Page>;
    async fn new_tab(&self, url: &str) -> Result<chromiumoxide::Page>;
    async fn get_page_hosts(&self) -> Result<Vec<String>>;
    async fn close_tabs_by_host(&self, host_substr: &str) -> Result<()>;
}

impl BrowserExt for Browser {
    async fn new_blank_tab(&self) -> Result<chromiumoxide::Page> {
        Ok(self
            .new_page(
                CreateTargetParams::builder()
                    .url("about:blank")
                    .background(true)
                    .build()
                    .map_err(|s| anyhow::anyhow!("{}", s))?,
            )
            .await?)
    }

    async fn new_tab(&self, url: &str) -> Result<chromiumoxide::Page> {
        let page = self.new_blank_tab().await?;
        page.goto(url).await?;
        Ok(page)
    }

    async fn get_page_hosts(&self) -> Result<Vec<String>> {
        let targets = self.execute(GetTargetsParams::default()).await?;
        Ok(targets
            .target_infos
            .iter()
            .filter(|t| t.r#type == "page")
            .filter_map(|t| host_of(&t.url))
            .collect())
    }

    async fn close_tabs_by_host(&self, host_substr: &str) -> Result<()> {
        let targets = self.execute(GetTargetsParams::default()).await?;
        for t in &targets.target_infos {
            if t.r#type == "page"
                && let Some(h) = host_of(&t.url)
                && h.contains(host_substr)
            {
                self.execute(CloseTargetParams {
                    target_id: t.target_id.clone(),
                })
                .await?;
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct BrowserManager {
    inner: Arc<Mutex<Option<Arc<Browser>>>>,
    handler: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl BrowserManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(None)),
            handler: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn ensure(&self) -> Result<Arc<Browser>> {
        let mut guard = self.inner.lock().await;

        if guard.is_none() {
            let (browser, handle) = match Self::connect().await {
                Ok(pair) => pair,
                Err(_) => Self::launch().await?,
            };
            *guard = Some(Arc::new(browser));
            *self.handler.lock().await = Some(handle);
        }

        Ok(guard.as_ref().unwrap().clone())
    }

    async fn connect() -> Result<(Browser, tokio::task::JoinHandle<()>)> {
        let (browser, mut handler) = Browser::connect(CDP_URL).await?;
        let handle = tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });
        Ok((browser, handle))
    }

    fn is_browser_running_without_cdp() -> bool {
        std::process::Command::new("pgrep")
            .arg("-x")
            .arg(BROWSER_APP)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    async fn launch() -> Result<(Browser, tokio::task::JoinHandle<()>)> {
        if Self::is_browser_running_without_cdp() {
            anyhow::bail!(
                "{BROWSER_APP} is running without remote debugging. Quit it and try again."
            );
        }

        let mut cmd = std::process::Command::new("open");
        cmd.arg("-g");
        cmd.arg("-a");
        cmd.arg(BROWSER_APP);
        cmd.arg("--args");
        cmd.arg("--remote-debugging-port=9222");
        cmd.spawn()?;

        let mut browser_and_handler = None;
        for _ in 0..30 {
            if let Ok(b) = Self::connect().await {
                browser_and_handler = Some(b);
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
        browser_and_handler.ok_or_else(|| anyhow::anyhow!("{BROWSER_APP} did not start in time"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_of_basic() {
        assert_eq!(
            host_of("https://upwork.com"),
            Some("upwork.com".to_string())
        );
        assert_eq!(
            host_of("https://www.upwork.com/jobs"),
            Some("upwork.com".to_string())
        );
        assert_eq!(
            host_of("https://NOFLUFFJOBS.COM/pl"),
            Some("nofluffjobs.com".to_string())
        );
    }

    #[test]
    fn test_host_of_malformed() {
        assert_eq!(host_of("not-a-url"), None);
        assert_eq!(host_of(""), None);
        assert_eq!(host_of("ftp://"), None);
    }

    #[test]
    fn test_host_of_with_port() {
        assert_eq!(
            host_of("http://localhost:9222"),
            Some("localhost".to_string())
        );
    }
}

impl Default for BrowserManager {
    fn default() -> Self {
        Self::new()
    }
}
