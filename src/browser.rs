use anyhow::{Result, bail};
use chromiumoxide::browser::Browser;
use chromiumoxide::cdp::browser_protocol::network::{CookieParam, CookieSameSite, TimeSinceEpoch};
use chromiumoxide::cdp::browser_protocol::target::{
    CloseTargetParams, CreateTargetParams, GetTargetsParams, TargetId,
};
use futures::StreamExt;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;

const CDP_URL: &str = "http://localhost:9222";
const BROWSER_APP: &str = "Brave Browser";

pub const DEFAULT_INIT_URLS: &[&str] = &[
    "https://www.upwork.com/freelancers/~01dba08086390dc196",
    "https://nofluffjobs.com",
    "https://www.efinancialcareers.com",
];

pub const REQUIRED_HOSTS: &[&str] = &["upwork.com", "nofluffjobs.com", "efinancialcareers.com"];

/// Open the given URLs in background tabs for any host that is not already open.
pub async fn ensure_init_tabs(browser: &Browser, urls: &[&str]) -> Result<()> {
    let page_urls = browser.get_page_urls().await?;
    for url in urls {
        let host = host_of(url);
        let has_tab = page_urls
            .iter()
            .filter_map(|u| host_of(u))
            .any(|h| Some(&h) == host.as_ref());
        if has_tab {
            continue;
        }
        let page = browser.new_blank_tab().await?;
        let _ = tokio::time::timeout(tokio::time::Duration::from_secs(5), page.goto(*url)).await;
    }
    Ok(())
}

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
    async fn get_page_urls(&self) -> Result<Vec<String>>;
    /// Return (target_id, url) pairs for all page targets.
    async fn get_page_targets(&self) -> Result<Vec<(TargetId, String)>>;
    /// Close page targets whose IDs are not in `keep_ids`.
    async fn close_pages_except(&self, keep_ids: &[TargetId]) -> Result<()>;
    /// Set a persistent, lax, root-path cookie for the given domain.
    async fn set_cookie(&self, name: &str, value: &str, domain: &str) -> Result<()>;
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

    async fn get_page_urls(&self) -> Result<Vec<String>> {
        Ok(self
            .get_page_targets()
            .await?
            .into_iter()
            .map(|(_, url)| url)
            .collect())
    }

    async fn get_page_targets(&self) -> Result<Vec<(TargetId, String)>> {
        let targets = self.execute(GetTargetsParams::default()).await?;
        Ok(targets
            .target_infos
            .iter()
            .filter(|t| t.r#type == "page")
            .map(|t| (t.target_id.clone(), t.url.clone()))
            .collect())
    }

    async fn close_pages_except(&self, keep_ids: &[TargetId]) -> Result<()> {
        let targets = self.get_page_targets().await?;
        for (id, _) in targets {
            if keep_ids.contains(&id) {
                continue;
            }
            let _ = self.execute(CloseTargetParams::new(id)).await;
        }
        Ok(())
    }

    async fn set_cookie(&self, name: &str, value: &str, domain: &str) -> Result<()> {
        let expires =
            TimeSinceEpoch::new(chrono::Utc::now().timestamp() as f64 + 365.0 * 24.0 * 60.0 * 60.0);
        let cookie = CookieParam::builder()
            .name(name)
            .value(value)
            .domain(domain)
            .path("/")
            .same_site(CookieSameSite::Lax)
            .expires(expires)
            .build()
            .map_err(|s| anyhow::anyhow!("{}", s))?;
        self.set_cookies(vec![cookie]).await?;
        Ok(())
    }
}

const DEFAULT_WAIT_DELAY_MS: u64 = 500;
const DEFAULT_WAIT_TRIES: u32 = 30;
const CHALLENGE_GRACE_PERIOD_SECS: u64 = 30;

/// Poll `condition` up to `tries` times, sleeping `delay` between attempts.
/// Returns `Ok(true)` as soon as the condition returns `Ok(true)`.
pub async fn wait_for<F, Fut>(
    condition: F,
    tries: Option<u32>,
    delay: Option<Duration>,
) -> Result<bool>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<bool>>,
{
    let tries = tries.unwrap_or(DEFAULT_WAIT_TRIES);
    let delay = delay.unwrap_or(Duration::from_millis(DEFAULT_WAIT_DELAY_MS));
    for _ in 0..tries {
        if condition().await? {
            return Ok(true);
        }
        sleep(delay).await;
    }
    Ok(false)
}

/// Wait until any of the `selectors` matches an element.
pub async fn wait_for_element(
    page: &chromiumoxide::Page,
    selectors: &[&str],
    tries: Option<u32>,
    delay: Option<Duration>,
) -> Result<bool> {
    wait_for(
        || async {
            for s in selectors {
                if page.find_element(*s).await.is_ok() {
                    return Ok(true);
                }
            }
            Ok(false)
        },
        tries,
        delay,
    )
    .await
}

/// Show an OS notification.
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn notify_user(title: &str, message: &str) {
    #[cfg(target_os = "macos")]
    {
        let script = format!("display notification {:?} with title {:?}", message, title);
        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("notify-send")
            .arg(title)
            .arg(message)
            .output();
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn notify_user(_title: &str, _message: &str) {}

/// Poll a JS condition until it returns true. If `challenge_js` is provided and
/// detects a bot challenge, send an OS notification and wait for the user to
/// solve it before resuming.
pub async fn wait_for_with_challenge_recovery(
    page: &chromiumoxide::Page,
    condition_js: &str,
    challenge_js: Option<&str>,
    tries: Option<u32>,
    delay: Option<Duration>,
    grace_period: Option<Duration>,
) -> Result<bool> {
    let tries = tries.unwrap_or(DEFAULT_WAIT_TRIES);
    let delay = delay.unwrap_or(Duration::from_millis(DEFAULT_WAIT_DELAY_MS));
    let grace = grace_period.unwrap_or(Duration::from_secs(CHALLENGE_GRACE_PERIOD_SECS));
    let mut notified = false;

    for _ in 0..tries {
        let matched: bool = page.evaluate(condition_js).await?.into_value()?;
        if matched {
            return Ok(true);
        }

        let Some(js) = challenge_js else {
            sleep(delay).await;
            continue;
        };

        let is_challenge: bool = page.evaluate(js).await?.into_value()?;
        if is_challenge {
            if !notified {
                notified = true;
                let url = page.url().await?.unwrap_or_default();
                notify_user(
                    "Jobsearch bot check",
                    &format!(
                        "{} hit a robot check. Solve it in the browser; test will resume.",
                        url
                    ),
                );
                eprintln!(
                    "Bot check detected at {}. Waiting {}s for user to solve...",
                    url,
                    grace.as_secs()
                );
            }
            sleep(grace).await;
            if page.evaluate(js).await?.into_value()? {
                bail!(
                    "Bot check still present after {}s. Solve it in the browser and retry.",
                    grace.as_secs()
                );
            }
            continue;
        }

        sleep(delay).await;
    }

    Ok(false)
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

    pub async fn browser(&self) -> Result<Arc<Browser>> {
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
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    async fn launch() -> Result<(Browser, tokio::task::JoinHandle<()>)> {
        if Self::is_browser_running_without_cdp() {
            Self::quit_browser().await?;
        }

        let _ = std::process::Command::new("open")
            .arg("-g")
            .arg("-a")
            .arg(BROWSER_APP)
            .arg("--args")
            .arg("--remote-debugging-port=9222")
            .output()?;

        let mut browser_and_handler = None;
        for _ in 0..30 {
            if let Ok(b) = Self::connect().await {
                browser_and_handler = Some(b);
                break;
            }
            sleep(Duration::from_millis(200)).await;
        }
        browser_and_handler.ok_or_else(|| anyhow::anyhow!("{BROWSER_APP} did not start in time"))
    }

    async fn quit_browser() -> Result<()> {
        eprintln!("{BROWSER_APP} is running without remote debugging; restarting it with CDP...");

        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg("quit app \"Brave Browser\"")
            .output()?;

        // Wait for the process to disappear.
        for _ in 0..30 {
            if !Self::is_browser_running_without_cdp() {
                return Ok(());
            }
            sleep(Duration::from_millis(200)).await;
        }
        anyhow::bail!("{BROWSER_APP} did not quit in time")
    }
}

impl Default for BrowserManager {
    fn default() -> Self {
        Self::new()
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
