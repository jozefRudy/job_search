mod browser;
mod cli;
mod db;
mod models;
mod platforms;

use anyhow::Result;
use browser::{BrowserExt, BrowserManager};
use clap::Parser;
use cli::{Cli, Commands};
use db::Db;
use directories::ProjectDirs;
use models::{JobStatus, Platform, Reaction};
use platforms::{PlatformClient, nofluffjobs::NoFluffJobsScraper, upwork::UpworkScraper};

const DEFAULT_INIT_URLS: &[&str] = &[
    "https://www.upwork.com/freelancers/~01dba08086390dc196",
    "https://nofluffjobs.com",
];



async fn cmd_init(browser: &BrowserManager, urls: &[&str]) -> Result<()> {
    eprintln!("Launching Brave browser with {} tabs...", urls.len());

    let browser = browser.ensure().await?;
    let hosts = browser.get_page_hosts().await?;

    for url in urls.iter() {
        let host = crate::browser::host_of(url);
        let has_tab = hosts.iter().any(|h| Some(h) == host.as_ref());
        if has_tab {
            eprintln!("  {} - already open, skipping", url);
            continue;
        }

        let page = browser.new_blank_tab().await?;
        match tokio::time::timeout(tokio::time::Duration::from_secs(3), page.goto(*url)).await {
            Ok(Ok(_)) => eprintln!("  {} - opened", url),
            _ => eprintln!("  {} - opened (loading...)", url),
        }
    }

    eprintln!("\nBrave is ready with {} tabs.", urls.len());
    eprintln!("Login to each site if needed, then run 'jobsearch update'.");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let db_path = cli.db.unwrap_or_else(|| {
        let dirs = ProjectDirs::from("", "", "jobsearch").expect("project dirs");
        dirs.data_dir().join("jobsearch.db")
    });

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = Db::open(&db_path).await?;
    let browser = BrowserManager::new();

    match cli.command {
        Commands::Init { urls } => {
            let init_urls: Vec<String> = match urls {
                Some(u) => u.clone(),
                None => DEFAULT_INIT_URLS.iter().map(|s| s.to_string()).collect(),
            };
            let init_urls: Vec<&str> = init_urls.iter().map(|s| s.as_str()).collect();
            cmd_init(&browser, &init_urls).await?;
        }
        Commands::Update { platform, query } => {
            cmd_update(&db, &browser, platform, &query).await?;
        }
        Commands::List {
            platform,
            status,
            limit,
        } => {
            cmd_list(&db, platform, status, limit, cli.json).await?;
        }
        Commands::Show { id } => {
            cmd_show(&db, id, cli.json).await?;
        }
        Commands::React { id, action } => {
            cmd_react(&db, id, action).await?;
        }
        Commands::Stats => {
            cmd_stats(&db, cli.json).await?;
        }
        Commands::Detail { id, force } => {
            cmd_detail(&db, &browser, id, force).await?;
        }
    }

    // Browser stays alive for reuse
    Ok(())
}

async fn cmd_update(
    db: &Db,
    browser: &BrowserManager,
    platform: Option<Platform>,
    query: &str,
) -> Result<()> {
    let clients: Vec<Box<dyn PlatformClient>> = match platform {
        Some(Platform::NoFluffJobs) => {
            vec![Box::new(NoFluffJobsScraper::new()) as Box<dyn PlatformClient>]
        }
        Some(Platform::Upwork) => {
            vec![Box::new(UpworkScraper::new()) as Box<dyn PlatformClient>]
        }
        None => vec![
            Box::new(NoFluffJobsScraper::new()) as Box<dyn PlatformClient>,
            Box::new(UpworkScraper::new()) as Box<dyn PlatformClient>,
        ],
    };

    for client in clients {
        eprintln!("Fetching from {}...", client.name());
        match client.fetch_with_manager(browser, query).await {
            Ok(jobs) => {
                eprintln!("  Found {} jobs", jobs.len());
                for job in &jobs {
                    db.upsert_job(job).await?;
                }
            }
            Err(e) => {
                eprintln!("  Error from {}: {}", client.name(), e);
            }
        }
    }

    Ok(())
}

async fn cmd_list(
    db: &Db,
    platform: Option<Platform>,
    status: Option<JobStatus>,
    limit: i64,
    json: bool,
) -> Result<()> {
    let jobs = db.list_jobs(platform, status, limit).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&jobs)?);
    } else {
        for job in &jobs {
            println!(
                "[{}] {} | {} | {} | {}",
                job.id.unwrap_or(0),
                job.platform,
                job.status.to_string().to_uppercase(),
                job.title,
                job.url
            );
        }
        println!("\nTotal: {} jobs", jobs.len());
    }

    Ok(())
}

async fn cmd_show(db: &Db, id: i64, json: bool) -> Result<()> {
    let job = db
        .get_job(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Job {} not found", id))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&job)?);
    } else {
        println!("ID:        {}", job.id.unwrap_or(0));
        println!("Platform:  {}", job.platform);
        println!("Status:    {}", job.status);
        println!("Title:     {}", job.title);
        println!("URL:       {}", job.url);
        println!(
            "Posted:    {}",
            job.posted_at
                .map(|d| d.to_rfc3339())
                .unwrap_or_else(|| "?".to_string())
        );
        println!("Budget:    {}", job.budget.as_deref().unwrap_or("?"));
        println!("Tags:      {}", job.tags.join(", "));
        println!("Desc:      {}", job.description.as_deref().unwrap_or("?"));

        // Show cached detail for Upwork
        if let Some(detail) = job.raw.get("detail") {
            let fetched = job
                .raw
                .get("detail_fetched_at")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            println!("\n--- Detail (fetched: {}) ---", fetched);
            println!(
                "Exact budget:   {}",
                detail
                    .get("exact_budget")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            );
            println!(
                "Proposals:      {}",
                detail
                    .get("proposals")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            );
            println!(
                "Last viewed:    {}",
                detail
                    .get("last_viewed")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            );
            println!(
                "Interviewing:   {}",
                detail
                    .get("interviewing")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            );
            println!(
                "Invites sent:   {}",
                detail
                    .get("invites_sent")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            );
            println!(
                "Unanswered:     {}",
                detail
                    .get("unanswered_invites")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            );
        }
    }

    Ok(())
}

async fn cmd_react(db: &Db, id: i64, action: Reaction) -> Result<()> {
    let status = match action {
        Reaction::Save => JobStatus::Saved,
        Reaction::Apply => JobStatus::Applied,
        Reaction::Hide => JobStatus::Hidden,
    };

    db.update_status(id, status).await?;
    db.add_reaction(id, action, None).await?;

    println!("Job {} marked as {:?}", id, status);
    Ok(())
}

async fn cmd_stats(db: &Db, json: bool) -> Result<()> {
    let stats = db.stats().await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&stats)?);
    } else {
        println!("Total jobs: {}", stats.total);
        println!("New jobs:   {}", stats.new_count);
        println!("\nBy status:");
        for (s, c) in &stats.by_status {
            println!("  {}: {}", s, c);
        }
        println!("\nBy platform:");
        for (p, c) in &stats.by_platform {
            println!("  {}: {}", p, c);
        }
    }

    Ok(())
}

async fn cmd_detail(db: &Db, browser: &BrowserManager, id: i64, force: bool) -> Result<()> {
    let job = db
        .get_job(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Job {} not found", id))?;

    match job.platform {
        Platform::Upwork => {
            let is_fresh = job.posted_at.is_some_and(|posted| {
                let age = chrono::Utc::now() - posted;
                age.num_days() < 7
            });

            let should_fetch = force || job.raw.get("detail").is_none() || is_fresh;

            let detail = if should_fetch {
                eprintln!("Fetching fresh detail...");
                let b = browser.ensure().await?;
                let scraper = UpworkScraper::new();
                let d = scraper.fetch_job_detail(&b, &job.url).await?;

                let mut raw = job.raw.clone();
                raw["detail"] = serde_json::to_value(&d)?;
                raw["detail_fetched_at"] = serde_json::to_value(chrono::Utc::now().to_rfc3339())?;
                db.update_raw(id, raw).await?;

                d
            } else {
                eprintln!("Using cached detail (use --force to refetch)");
                serde_json::from_value::<crate::platforms::upwork::JobDetail>(
                    job.raw["detail"].clone(),
                )?
            };

            println!("Title:          {}", job.title);
            println!("Budget:         {}", detail.exact_budget);
            println!("Proposals:      {}", detail.proposals);
            println!("Last viewed:    {}", detail.last_viewed);
            println!("Interviewing:   {}", detail.interviewing);
            println!("Invites sent:   {}", detail.invites_sent);
            println!("Unanswered:     {}", detail.unanswered_invites);
            println!("\nDescription:\n{}", detail.description);
        }
        Platform::NoFluffJobs => {
            println!("Detail fetch not yet supported for NoFluffJobs");
        }
    }

    Ok(())
}
