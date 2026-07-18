use chromiumoxide::browser::Browser;
use futures::FutureExt;
use jobsearch::browser::{BrowserExt, BrowserManager, DEFAULT_INIT_URLS};
use jobsearch::language::LanguageService;
use jobsearch::platforms::linkedin::{LinkedInScraper, fetch_job_detail};
use jobsearch::platforms::upwork::UpworkScraper;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::Mutex;

/// Serialize access to shared Brave browser.
static BROWSER_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

async fn with_browser<F, Fut>(timeout_secs: u64, f: F)
where
    F: FnOnce(std::sync::Arc<Browser>) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let _guard = BROWSER_LOCK.lock().await;
    tokio::time::timeout(Duration::from_secs(timeout_secs), async {
        let manager = BrowserManager::new();
        let browser = manager.browser().await.expect("Brave should connect");
        jobsearch::browser::ensure_init_tabs(&browser, DEFAULT_INIT_URLS)
            .await
            .expect("ensure_init_tabs should succeed");
        let initial = browser
            .get_page_targets()
            .await
            .expect("snapshot initial tabs");
        let initial_ids: Vec<_> = initial.into_iter().map(|(id, _)| id).collect();

        let result = std::panic::AssertUnwindSafe(f(browser.clone()))
            .catch_unwind()
            .await;

        browser
            .close_pages_except(&initial_ids)
            .await
            .expect("close test tabs");

        if let Err(err) = result {
            std::panic::resume_unwind(err);
        }
    })
    .await
    .expect("test should complete within timeout");
}

fn efc_search_url(keyword: &str) -> String {
    let keyword = keyword.trim();
    let encoded = keyword.replace(' ', "+");
    format!(
        "https://www.efinancialcareers.com/jobs/remote?radius=50&radiusUnit=mi&pageSize=10&filters.workArrangementType=REMOTE&currencyCode=USD&filters.minSalary=100000&language=en&q={encoded}&includeUnspecifiedSalary=true&enableVectorSearch=true",
    )
}

// --- Hacker News: fetch jobs via Algolia (no browser) ---

#[tokio::test]
#[ignore = "requires network access to Hacker News Algolia API"]
async fn test_hackernews_fetch_comments() {
    let scraper = jobsearch::platforms::hackernews::HackerNewsScraper::new(
        None,
        "Europe",
        "https://hn.algolia.com/api/v1/search_by_date",
    )
    .expect("HackerNewsScraper should be created");
    let comments = scraper
        .fetch_top_level_comments("rust", Some(5))
        .await
        .expect("fetch_top_level_comments should succeed");

    assert!(!comments.is_empty(), "should find at least one HN comment");

    let first = &comments[0];
    assert!(!first.comment_text.is_empty(), "comment_text required");

    println!(
        "found {} HN comments, first text length: {}",
        comments.len(),
        first.comment_text.len()
    );
}

#[tokio::test]
#[ignore = "requires Brave browser installed and upwork.com logged in"]
async fn test_upwork_search_page_has_cards() {
    with_browser(45, |browser| async move {
        let search_url =
            "https://www.upwork.com/nx/search/jobs/?q=rust&sort=recency&per_page=50&t=0";
        let page = browser.new_tab(search_url).await.expect("open search page");

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
        println!(
            "{}",
            serde_json::to_string_pretty(first).expect("serialize job")
        );

        page.close().await.ok();
    })
    .await;
}

// --- Upwork: job detail ---

#[tokio::test]
#[ignore = "requires Brave browser installed and upwork.com logged in"]
async fn test_upwork_job_detail_fetch() {
    with_browser(60, |browser| async move {
        // Grab a job URL from search
        let search_url =
            "https://www.upwork.com/nx/search/jobs/?q=rust&sort=recency&per_page=50&t=0";
        let page = browser.new_tab(search_url).await.expect("open search");

        assert!(
            UpworkScraper::wait_for_jobs(&page)
                .await
                .expect("wait_for_jobs"),
            "search page should load"
        );

        let jobs = UpworkScraper::scrape_page(&page).await.expect("scrape");
        assert!(!jobs.is_empty(), "need at least one job for detail test");
        let job_url = jobs[0].url.clone();
        println!("fetching detail for: {job_url}");
        page.close().await.ok();

        let scraper = UpworkScraper::new();
        let detail = scraper
            .fetch_job_detail(browser.as_ref(), job_url.as_str())
            .await
            .expect("fetch_job_detail should succeed");

        assert!(
            !detail.description.is_empty(),
            "detail should have description"
        );
        assert!(
            !detail.exact_budget.is_empty(),
            "detail should have exact_budget or a hidden-budget marker"
        );
        assert!(
            !detail.experience_level.is_empty(),
            "detail should have experience_level"
        );
        assert!(!detail.duration.is_empty(), "detail should have duration");
        assert!(
            !detail.hours_per_week.is_empty(),
            "detail should have hours_per_week"
        );
        assert!(!detail.tags.is_empty(), "detail should have tags");
        println!("detail struct:");
        println!(
            "{}",
            serde_json::to_string_pretty(&detail).expect("serialize detail")
        );
    })
    .await;
}

// --- Upwork: pagination ---

#[tokio::test]
#[ignore = "requires Brave browser installed and upwork.com logged in"]
async fn test_upwork_pagination_has_next_page() {
    with_browser(60, |browser| async move {
        let search_url =
            "https://www.upwork.com/nx/search/jobs/?q=rust&sort=recency&per_page=50&t=0";
        let page = browser.new_tab(search_url).await.expect("open search page");

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
        println!("first page: {first_count} jobs");

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

        let next_url =
            "https://www.upwork.com/nx/search/jobs/?q=rust&sort=recency&per_page=50&t=0&page=2";
        page.goto(next_url).await.expect("goto page 2");
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
        println!("second page: {second_count} jobs");

        assert!(!second_page.is_empty(), "page 2 should have jobs");

        page.close().await.ok();
    })
    .await;
}

// --- NoFluffJobs: search page ---

#[tokio::test]
#[ignore = "requires Brave browser installed and nofluffjobs.com accessible"]
async fn test_nofluffjobs_search_page_has_cards_and_details() {
    with_browser(45, |browser| async move {
        let scraper =
            jobsearch::platforms::nofluffjobs::NoFluffJobsScraper::new(LanguageService::new());
        let search_url = "https://nofluffjobs.com/remote?criteria=keyword%3Drust&sort=newest";
        let page = browser.new_tab(search_url).await.expect("open search page");

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
    .await;
}

// --- NoFluffJobs: load more ---

#[tokio::test]
#[ignore = "requires Brave browser installed and nofluffjobs.com accessible"]
async fn test_nofluffjobs_load_more_adds_jobs() {
    with_browser(60, |browser| async move {
        let search_url = "https://nofluffjobs.com/remote?criteria=keyword%3Drust&sort=newest";
        let page = browser.new_tab(search_url).await.expect("open search page");

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
        println!("first page: {first_count} jobs");

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
        println!("after load-more: {total} jobs");

        assert!(
            total > first_count,
            "after load-more, total should increase: {total} vs {first_count}"
        );

        page.close().await.ok();
    })
    .await;
}

// --- eFinancialCareers: search page ---

#[tokio::test]
#[ignore = "requires Brave browser installed and efinancialcareers.com accessible"]
async fn test_efinancialcareers_search_page_has_cards_and_details() {
    with_browser(45, |browser| async move {
        let scraper = jobsearch::platforms::efinancialcareers::EfinancialcareersScraper::new(
            LanguageService::new(),
        );
        let search_url = efc_search_url("developer");
        let page = browser
            .new_tab(&search_url)
            .await
            .expect("open search page");

        let ok =
            jobsearch::platforms::efinancialcareers::EfinancialcareersScraper::wait_for_jobs(&page)
                .await
                .expect("wait_for_jobs should not error");
        assert!(ok, "jobs should appear on search page");

        let jobs =
            jobsearch::platforms::efinancialcareers::EfinancialcareersScraper::scrape_page(&page)
                .await
                .expect("scrape_page should not error");
        assert!(!jobs.is_empty(), "should find at least one job card");

        let total_jobs =
            jobsearch::platforms::efinancialcareers::EfinancialcareersScraper::scrape_total(&page)
                .await
                .expect("scrape_total should find a count in heading");
        assert!(total_jobs > 0, "total job count should be positive");
        println!("total jobs from heading: {total_jobs}");

        let first = &jobs[0];
        assert!(!first.external_id.is_empty(), "external_id required");
        assert!(!first.title.is_empty(), "title required");
        assert!(
            first.url.starts_with("https://www.efinancialcareers.com/"),
            "url must be on efinancialcareers.com: {}",
            first.url
        );
        println!("found {} cards, first card:", jobs.len());
        println!(
            "{}",
            serde_json::to_string_pretty(first).expect("serialization should succeed")
        );

        let http = reqwest::Client::new();
        let detail = scraper
            .fetch_detail(&http, first.external_id.as_str())
            .await
            .expect("fetch_detail should succeed");
        assert!(
            !detail.description.is_empty(),
            "detail should have description"
        );
        assert!(
            !detail.company.is_empty(),
            "detail should have company: {} - add selector if missing",
            first.url
        );
        assert!(
            !detail.location.is_empty(),
            "detail should have location: {} - add selector if missing",
            first.url
        );

        println!("detail struct:");
        println!(
            "{}",
            serde_json::to_string_pretty(&detail).expect("serialization should succeed")
        );

        page.close().await.ok();
    })
    .await;
}

// --- eFinancialCareers: load more ---

#[tokio::test]
#[ignore = "requires Brave browser installed and efinancialcareers.com accessible"]
async fn test_efinancialcareers_show_more_adds_jobs() {
    with_browser(60, |browser| async move {
        let search_url = efc_search_url("");
        let page = browser
            .new_tab(&search_url)
            .await
            .expect("open search page");

        assert!(
            jobsearch::platforms::efinancialcareers::EfinancialcareersScraper::wait_for_jobs(&page)
                .await
                .expect("wait_for_jobs"),
            "jobs should appear"
        );
        tokio::time::sleep(Duration::from_secs(2)).await;

        let first_page =
            jobsearch::platforms::efinancialcareers::EfinancialcareersScraper::scrape_page(&page)
                .await
                .expect("scrape first page");
        assert!(!first_page.is_empty(), "first page should have jobs");
        let first_count = first_page.len();
        println!("first page: {first_count} jobs");

        let more_loaded =
            jobsearch::platforms::efinancialcareers::EfinancialcareersScraper::click_show_more(
                &page, 2000,
            )
            .await;
        if !more_loaded {
            println!("No 'Show more' button or no more jobs — skipping assertion");
            page.close().await.ok();
            return;
        }

        let second_page =
            jobsearch::platforms::efinancialcareers::EfinancialcareersScraper::scrape_page(&page)
                .await
                .expect("scrape second page");
        let total = second_page.len();
        println!("after show-more: {total} jobs");

        assert!(
            total > first_count,
            "after show-more, total should increase: {total} vs {first_count}"
        );

        page.close().await.ok();
    })
    .await;
}

#[tokio::test]
#[ignore = "requires Brave browser installed and efinancialcareers.com accessible"]
async fn test_efinancialcareers_zero_results_returns_count_zero() {
    with_browser(45, |browser| async move {
        let search_url = efc_search_url("xyznonexistent12345thisshouldreturnnojobs");
        let page = browser
            .new_tab(&search_url)
            .await
            .expect("open search page");

        assert!(
            jobsearch::platforms::efinancialcareers::EfinancialcareersScraper::wait_for_jobs(&page)
                .await
                .expect("wait_for_jobs should not error"),
            "page should render job cards section even for no matches"
        );

        let total_jobs =
            jobsearch::platforms::efinancialcareers::EfinancialcareersScraper::scrape_total(&page)
                .await
                .expect("scrape_total should not error on zero-results page");
        assert_eq!(total_jobs, 0, "total job count should be 0 for no matches");

        page.close().await.ok();
    })
    .await;
}

// --- LinkedIn: search page ---

#[tokio::test]
#[ignore = "requires Brave browser installed and linkedin.com logged in"]
async fn test_linkedin_search_page_has_cards() {
    with_browser(60, |browser| async move {
        let scraper = LinkedInScraper::new("https://www.linkedin.com/jobs/search/");
        let search_url = "https://www.linkedin.com/jobs/search/";
        let page = browser
            .new_tab(search_url)
            .await
            .expect("open LinkedIn search page");

        assert!(
            LinkedInScraper::wait_for_jobs(&page)
                .await
                .expect("wait_for_jobs should not error"),
            "jobs should appear on search page"
        );

        let cards = scraper.scrape_page(&page).await.expect("scrape_page");
        assert!(!cards.is_empty(), "should find at least one job card");

        let first = &cards[0];
        assert!(!first.id.is_empty(), "external_id required");
        assert!(!first.title.is_empty(), "title required");
        assert!(
            first.id.parse::<u64>().is_ok(),
            "id should be numeric: {}",
            first.id
        );

        println!("found {} cards, first card:", cards.len());
        println!(
            "{}",
            serde_json::to_string_pretty(first).expect("serialize first card")
        );

        page.close().await.ok();
    })
    .await;
}

// --- LinkedIn: job detail ---

#[tokio::test]
#[ignore = "requires Brave browser installed and linkedin.com logged in"]
async fn test_linkedin_job_detail_fetch() {
    with_browser(60, |browser| async move {
        let scraper = LinkedInScraper::new("https://www.linkedin.com/jobs/search/");
        let search_url = "https://www.linkedin.com/jobs/search/";
        let page = browser
            .new_tab(search_url)
            .await
            .expect("open LinkedIn search");

        assert!(
            LinkedInScraper::wait_for_jobs(&page)
                .await
                .expect("wait_for_jobs"),
            "search page should load"
        );

        let cards = scraper.scrape_page(&page).await.expect("scrape");
        assert!(!cards.is_empty(), "need at least one job for detail test");
        let job_id: u64 = cards[0].id.parse().expect("parse job id");
        println!("fetching detail for job id: {job_id}");

        let detail = fetch_job_detail(&page, job_id)
            .await
            .expect("fetch_job_detail should succeed");

        assert!(
            !detail.description.is_empty(),
            "detail should have description"
        );
        assert!(!detail.company.is_empty(), "detail should have company");
        assert!(!detail.location.is_empty(), "detail should have location");

        println!("detail struct:");
        println!(
            "{}",
            serde_json::to_string_pretty(&detail).expect("serialize detail")
        );

        page.close().await.ok();
    })
    .await;
}

// --- LinkedIn: pagination ---

#[tokio::test]
#[ignore = "requires Brave browser installed and linkedin.com logged in"]
async fn test_linkedin_pagination_has_next_page() {
    with_browser(60, |browser| async move {
        let scraper = LinkedInScraper::new("https://www.linkedin.com/jobs/search/");
        let search_url = "https://www.linkedin.com/jobs/search/";
        let page = browser
            .new_tab(search_url)
            .await
            .expect("open LinkedIn search page");

        assert!(
            LinkedInScraper::wait_for_jobs(&page)
                .await
                .expect("wait_for_jobs"),
            "jobs should appear"
        );

        let first_page = scraper.scrape_page(&page).await.expect("scrape first page");
        assert!(!first_page.is_empty(), "first page should have jobs");
        let first_count = first_page.len();
        let total = scraper
            .fetch_page(&page, 0)
            .await
            .expect("fetch page 0")
            .total;
        println!("first page: {first_count} jobs, total: {total}");

        if total as usize <= first_count {
            println!("Not enough jobs for pagination — skipping");
            page.close().await.ok();
            return;
        }

        let second_page = scraper
            .fetch_page(&page, first_count)
            .await
            .expect("fetch page 2")
            .cards;
        assert!(!second_page.is_empty(), "page 2 should have jobs");
        assert_ne!(
            first_page[0].id, second_page[0].id,
            "page 2 should start with a different job"
        );

        page.close().await.ok();
    })
    .await;
}
