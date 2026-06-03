use jobsearch::browser::{BrowserExt, BrowserManager};
use jobsearch::platforms::{nofluffjobs::NoFluffJobsScraper, upwork::UpworkScraper};
use std::sync::Mutex;
use std::time::Duration;

/// Serialize access to shared Brave browser.
static BROWSER_LOCK: Mutex<()> = Mutex::new(());

fn get_guard() -> std::sync::MutexGuard<'static, ()> {
    loop {
        match BROWSER_LOCK.lock() {
            Ok(g) => return g,
            Err(_) => {
                let mut guard = BROWSER_LOCK.lock().unwrap();
                *guard = ();
            }
        }
    }
}

#[tokio::test]
#[ignore = "requires Brave browser installed"]
async fn test_browser_manager_connects_or_launches() {
    let _guard = get_guard();
    let manager = BrowserManager::new();

    tokio::time::timeout(Duration::from_secs(10), async {
        let browser = manager
            .ensure()
            .await
            .expect("Brave should connect or launch");
        let hosts = browser
            .get_page_hosts()
            .await
            .expect("get_page_hosts should work");
        println!("page hosts: {:?}", hosts);
    })
    .await
    .expect("test should complete within 10s");
}

// --- Upwork: search page ---

#[tokio::test]
#[ignore = "requires Brave browser installed and upwork.com logged in"]
async fn test_upwork_search_page_has_cards() {
    let _guard = get_guard();
    let manager = BrowserManager::new();

    tokio::time::timeout(Duration::from_secs(45), async {
        let browser = manager.ensure().await.expect("Brave should connect");
        let search_url = UpworkScraper::build_search_url("rust", None, None);
        let page = browser
            .new_tab(&search_url)
            .await
            .expect("open search page");

        let ok = UpworkScraper::wait_for_jobs(&page)
            .await
            .expect("wait_for_jobs should not error");
        assert!(ok, "jobs should appear on search page");

        let jobs = UpworkScraper::scrape_page(&page)
            .await
            .expect("scrape_page should not error");
        assert!(!jobs.is_empty(), "should find at least one job card");

        let first = &jobs[0];
        assert!(!first.external_id.is_empty(), "external_id required");
        assert!(!first.title.is_empty(), "title required");
        assert!(
            first.url.starts_with("https://www.upwork.com/"),
            "url must be on upwork.com: {}",
            first.url
        );
        println!("found {} cards, first card:", jobs.len());
        println!("{}", serde_json::to_string_pretty(first).unwrap());

        page.close().await.ok();
    })
    .await
    .expect("test should complete within 45s");
}

// --- Upwork: job detail ---

#[tokio::test]
#[ignore = "requires Brave browser installed and upwork.com logged in"]
async fn test_upwork_job_detail_fetch() {
    let _guard = get_guard();
    let manager = BrowserManager::new();

    tokio::time::timeout(Duration::from_secs(60), async {
        let browser = manager.ensure().await.expect("Brave should connect");

        // Grab a job URL from search
        let search_url = UpworkScraper::build_search_url("rust", None, None);
        let page = browser.new_tab(&search_url).await.expect("open search");

        assert!(
            UpworkScraper::wait_for_jobs(&page)
                .await
                .expect("wait_for_jobs"),
            "search page should load"
        );

        let jobs = UpworkScraper::scrape_page(&page).await.expect("scrape");
        assert!(!jobs.is_empty(), "need at least one job for detail test");
        let job_url = jobs[0].url.clone();
        println!("fetching detail for: {}", job_url);
        page.close().await.ok();

        let scraper = UpworkScraper::new();
        let detail = scraper
            .fetch_job_detail(&browser, &job_url)
            .await
            .expect("fetch_job_detail should succeed");

        assert!(
            !detail.description.is_empty(),
            "detail should have description"
        );
        println!("detail struct:");
        println!("{}", serde_json::to_string_pretty(&detail).unwrap());
    })
    .await
    .expect("test should complete within 60s");
}

// --- Upwork: load more ---

#[tokio::test]
#[ignore = "requires Brave browser installed and upwork.com logged in"]
async fn test_upwork_load_more_adds_jobs() {
    let _guard = get_guard();
    let manager = BrowserManager::new();

    tokio::time::timeout(Duration::from_secs(60), async {
        let browser = manager.ensure().await.expect("Brave should connect");
        let search_url = UpworkScraper::build_search_url("rust", None, None);
        let page = browser
            .new_tab(&search_url)
            .await
            .expect("open search page");

        assert!(
            UpworkScraper::wait_for_jobs(&page)
                .await
                .expect("wait_for_jobs"),
            "jobs should appear"
        );
        tokio::time::sleep(Duration::from_secs(2)).await;

        let first_page = UpworkScraper::scrape_page(&page)
            .await
            .expect("scrape first page");
        assert!(!first_page.is_empty(), "first page should have jobs");
        let first_count = first_page.len();
        println!("first page: {} jobs", first_count);

        let more_loaded = UpworkScraper::click_load_more(&page, 2000).await;
        if !more_loaded {
            println!("No load-more button or no more jobs — skipping assertion");
            page.close().await.ok();
            return;
        }

        let second_page = UpworkScraper::scrape_page(&page)
            .await
            .expect("scrape second page");
        let total = second_page.len();
        println!("after load-more: {} jobs", total);

        assert!(
            total > first_count,
            "after load-more, total should increase: {} vs {}",
            total,
            first_count
        );

        page.close().await.ok();
    })
    .await
    .expect("test should complete within 60s");
}

// --- NoFluffJobs: search page ---

#[tokio::test]
#[ignore = "requires Brave browser installed and nofluffjobs.com logged in"]
async fn test_nofluffjobs_search_page_has_cards() {
    let _guard = get_guard();
    let manager = BrowserManager::new();

    tokio::time::timeout(Duration::from_secs(45), async {
        let browser = manager.ensure().await.expect("Brave should connect");
        let scraper = NoFluffJobsScraper::new();
        let search_url = scraper.build_search_url("rust");
        let page = browser
            .new_tab(&search_url)
            .await
            .expect("open search page");

        let ok = NoFluffJobsScraper::wait_for_jobs(&page)
            .await
            .expect("wait_for_jobs should not error");
        assert!(ok, "jobs should appear on search page");

        let jobs = NoFluffJobsScraper::scrape_page(&page)
            .await
            .expect("scrape_page should not error");
        assert!(!jobs.is_empty(), "should find at least one job card");

        let first = &jobs[0];
        assert!(!first.external_id.is_empty(), "external_id required");
        assert!(!first.title.is_empty(), "title required");
        assert!(
            first.url.starts_with("https://nofluffjobs.com/"),
            "url must be on nofluffjobs.com: {}",
            first.url
        );
        println!("found {} cards, first card:", jobs.len());
        println!("{}", serde_json::to_string_pretty(first).unwrap());

        page.close().await.ok();
    })
    .await
    .expect("test should complete within 45s");
}

// --- NoFluffJobs: job detail ---

#[tokio::test]
#[ignore = "requires Brave browser installed and nofluffjobs.com logged in"]
async fn test_nofluffjobs_job_detail_fetch() {
    let _guard = get_guard();
    let manager = BrowserManager::new();

    tokio::time::timeout(Duration::from_secs(60), async {
        let browser = manager.ensure().await.expect("Brave should connect");
        let scraper = NoFluffJobsScraper::new();

        let search_url = scraper.build_search_url("rust");
        let page = browser.new_tab(&search_url).await.expect("open search");

        assert!(
            NoFluffJobsScraper::wait_for_jobs(&page)
                .await
                .expect("wait_for_jobs"),
            "search page should load"
        );

        let jobs = NoFluffJobsScraper::scrape_page(&page)
            .await
            .expect("scrape");
        assert!(!jobs.is_empty(), "need at least one job for detail test");
        let job_url = jobs[0].url.clone();
        println!("fetching detail for: {}", job_url);
        page.close().await.ok();

        let detail = scraper
            .fetch_job_detail(&browser, &job_url)
            .await
            .expect("fetch_job_detail should succeed");

        assert!(!detail.company.is_empty(), "detail should have company");
        println!("detail struct:");
        println!("{}", serde_json::to_string_pretty(&detail).unwrap());
    })
    .await
    .expect("test should complete within 60s");
}
