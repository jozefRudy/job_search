use jobsearch::browser::{BrowserExt, BrowserManager};
use jobsearch::models::Platform;
use jobsearch::platforms::{
    PlatformClient, nofluffjobs::NoFluffJobsScraper, upwork::UpworkScraper,
};

#[tokio::test]
#[ignore = "requires Brave browser installed"]
async fn test_browser_manager_connects_or_launches() {
    let manager = BrowserManager::new();
    let browser = manager
        .ensure()
        .await
        .expect("Brave should connect or launch");

    let hosts = browser
        .get_page_hosts()
        .await
        .expect("get_page_hosts should work");
    println!("page hosts: {:?}", hosts);
}

#[tokio::test]
#[ignore = "requires Brave browser installed and nofluffjobs.com logged in"]
async fn test_nofluffjobs_fetches_jobs_with_tab() {
    let manager = BrowserManager::new();
    let browser = manager
        .ensure()
        .await
        .expect("Brave should connect or launch");

    let scraper = NoFluffJobsScraper::new();
    let jobs = scraper
        .fetch_with_browser(&browser, "rust")
        .await
        .expect("fetch should not error");

    assert!(!jobs.is_empty(), "Should find jobs when tab is open");
    assert!(
        jobs.iter().all(|j| j.platform == Platform::NoFluffJobs),
        "All jobs should be NoFluffJobs"
    );
}

#[tokio::test]
#[ignore = "requires Brave browser installed and upwork.com logged in"]
async fn test_upwork_fetches_jobs_with_tab() {
    let manager = BrowserManager::new();
    let browser = manager
        .ensure()
        .await
        .expect("Brave should connect or launch");

    let scraper = UpworkScraper::new();
    let jobs = scraper
        .fetch_with_browser(&browser, "rust")
        .await
        .expect("fetch should not error");

    assert!(!jobs.is_empty(), "Should find jobs when tab is open");
    assert!(
        jobs.iter().all(|j| j.platform == Platform::Upwork),
        "All jobs should be Upwork"
    );
}
