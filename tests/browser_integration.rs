use jobsearch::browser::{BrowserExt, BrowserManager};

#[tokio::test]
#[ignore = "requires Brave browser installed"]
async fn test_browser_manager_connects_or_launches() {
    let manager = BrowserManager::new();
    let browser = manager
        .ensure()
        .await
        .expect("Brave should connect or launch");

    let hosts = browser.get_page_hosts().await.unwrap();
    println!("page hosts: {:?}", hosts);
}
