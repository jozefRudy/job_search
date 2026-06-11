use jobsearch::browser::{BrowserExt, BrowserManager};
use jobsearch::platforms::PlatformClient;
use jobsearch::platforms::upwork::UpworkScraper;
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
        let search_url = UpworkScraper::build_search_url("rust", None, None, None, 1);
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
        let search_url = UpworkScraper::build_search_url("rust", None, None, None, 1);
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

// --- Upwork: pagination ---

#[tokio::test]
#[ignore = "requires Brave browser installed and upwork.com logged in"]
async fn test_upwork_pagination_has_next_page() {
    let _guard = get_guard();
    let manager = BrowserManager::new();

    tokio::time::timeout(Duration::from_secs(60), async {
        let browser = manager.ensure().await.expect("Brave should connect");
        let search_url = UpworkScraper::build_search_url("rust", None, None, None, 1);
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

        let has_next: bool = page
            .evaluate(r#"!!document.querySelector('a[data-test="next-page"]:not(.is-disabled)')"#)
            .await
            .ok()
            .and_then(|v| v.into_value().ok())
            .unwrap_or(false);

        if !has_next {
            println!("No next page — skipping pagination assertion");
            page.close().await.ok();
            return;
        }

        let next_url = UpworkScraper::build_search_url("rust", None, None, None, 2);
        page.goto(&next_url).await.expect("goto page 2");
        page.wait_for_navigation().await.expect("navigation");

        assert!(
            UpworkScraper::wait_for_jobs(&page)
                .await
                .expect("wait_for_jobs page 2"),
            "jobs should appear on page 2"
        );
        tokio::time::sleep(Duration::from_secs(2)).await;

        let second_page = UpworkScraper::scrape_page(&page)
            .await
            .expect("scrape second page");
        let second_count = second_page.len();
        println!("second page: {} jobs", second_count);

        assert!(!second_page.is_empty(), "page 2 should have jobs");

        page.close().await.ok();
    })
    .await
    .expect("test should complete within 60s");
}

// --- Upwork: sync applications ---

#[tokio::test]
#[ignore = "requires Brave browser installed and upwork.com logged in"]
async fn test_upwork_sync_applications() {
    let _guard = get_guard();
    let manager = BrowserManager::new();

    tokio::time::timeout(Duration::from_secs(120), async {
        let browser = manager.ensure().await.expect("Brave should connect");
        let tmp = tempfile::NamedTempFile::new().expect("temp db");
        let db = jobsearch::db::Db::open(tmp.path()).await.expect("open db");
        let scraper = UpworkScraper::new();

        let synced = scraper
            .sync_applications(&browser, &db, 500, Some(1))
            .await
            .expect("sync_applications should succeed");

        println!("Synced {} applications", synced);

        if synced == 0 {
            println!("No new submitted proposals found — skipping DB assertions");
            return;
        }

        let jobs = db
            .list_jobs(
                Some(jobsearch::models::Platform::Upwork),
                jobsearch::models::Sort::Created,
                i64::MAX,
            )
            .await
            .expect("list jobs");

        let applied_jobs: Vec<_> = jobs
            .into_iter()
            .filter(|j| j.applied_at.is_some())
            .collect();
        assert!(
            !applied_jobs.is_empty(),
            "at least one job should have applied_at set"
        );

        for job in &applied_jobs {
            println!(
                "applied: {} | applied_at: {:?} | note_len: {:?}",
                job.title,
                job.applied_at,
                job.note.as_ref().map(|n| n.len())
            );
        }
    })
    .await
    .expect("test should complete within 120s");
}

// --- NoFluffJobs: search page ---

#[tokio::test]
#[ignore = "requires Brave browser installed and nofluffjobs.com accessible"]
async fn test_nofluffjobs_search_page_has_cards_and_details() {
    let _guard = get_guard();
    let manager = BrowserManager::new();

    tokio::time::timeout(Duration::from_secs(45), async {
        let browser = manager.ensure().await.expect("Brave should connect");
        let scraper = jobsearch::platforms::nofluffjobs::NoFluffJobsScraper::new();
        let search_url = scraper.build_search_url("rust");
        let page = browser
            .new_tab(&search_url)
            .await
            .expect("open search page");

        let ok = jobsearch::platforms::nofluffjobs::NoFluffJobsScraper::wait_for_jobs(&page)
            .await
            .expect("wait_for_jobs should not error");
        assert!(ok, "jobs should appear on search page");

        let jobs = jobsearch::platforms::nofluffjobs::NoFluffJobsScraper::scrape_page(&page)
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
        println!(
            "{}",
            serde_json::to_string_pretty(first).expect("serialization should succeed")
        );

        let detail = scraper
            .fetch_detail(&first.external_id)
            .await
            .expect("fetch_detail should succeed");

        println!("detail struct:");
        println!(
            "{}",
            serde_json::to_string_pretty(&detail).expect("serialization should succeed")
        );

        page.close().await.ok();
    })
    .await
    .expect("test should complete within 45s");
}

// --- NoFluffJobs: load more ---

#[tokio::test]
#[ignore = "requires Brave browser installed and nofluffjobs.com accessible"]
async fn test_nofluffjobs_load_more_adds_jobs() {
    let _guard = get_guard();
    let manager = BrowserManager::new();

    tokio::time::timeout(Duration::from_secs(60), async {
        let browser = manager.ensure().await.expect("Brave should connect");
        let scraper = jobsearch::platforms::nofluffjobs::NoFluffJobsScraper::new();
        let search_url = scraper.build_search_url("rust");
        let page = browser
            .new_tab(&search_url)
            .await
            .expect("open search page");

        assert!(
            jobsearch::platforms::nofluffjobs::NoFluffJobsScraper::wait_for_jobs(&page)
                .await
                .expect("wait_for_jobs"),
            "jobs should appear"
        );
        tokio::time::sleep(Duration::from_secs(2)).await;

        let first_page = jobsearch::platforms::nofluffjobs::NoFluffJobsScraper::scrape_page(&page)
            .await
            .expect("scrape first page");
        assert!(!first_page.is_empty(), "first page should have jobs");
        let first_count = first_page.len();
        println!("first page: {} jobs", first_count);

        let more_loaded =
            jobsearch::platforms::nofluffjobs::NoFluffJobsScraper::click_load_more(&page, 2000)
                .await;
        if !more_loaded {
            println!("No 'See more offers' button or no more jobs — skipping assertion");
            page.close().await.ok();
            return;
        }

        let second_page = jobsearch::platforms::nofluffjobs::NoFluffJobsScraper::scrape_page(&page)
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
