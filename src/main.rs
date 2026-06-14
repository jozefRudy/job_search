use anyhow::Result;
use clap::Parser;
use directories::ProjectDirs;
use jobsearch::browser::{BrowserExt, BrowserManager, DEFAULT_INIT_URLS, ensure_init_tabs};
use jobsearch::cli::{
    Cli, Commands, ListTarget, ReactAction, SyncPlatform, UpdatePlatform, UpworkSortBy,
};
use jobsearch::db::Db;
use jobsearch::display;
use jobsearch::models::{JobFilter, Platform, Sort};
use jobsearch::platforms::{
    PlatformClient,
    efinancialcareers::{EfinancialcareersConfig, EfinancialcareersScraper},
    nofluffjobs::NoFluffJobsScraper,
    upwork::UpworkScraper,
};
use jobsearch::server;

async fn cmd_init(browser: &BrowserManager, urls: &[&str]) -> Result<()> {
    eprintln!("Launching Brave browser with {} tabs...", urls.len());

    let browser = browser.ensure().await?;
    let tabs_before = browser.get_page_urls().await?;
    ensure_init_tabs(&browser, urls).await?;
    let tabs_after = browser.get_page_urls().await?;

    for url in urls.iter() {
        let host = jobsearch::browser::host_of(url);
        let was_open = tabs_before
            .iter()
            .filter_map(|u| jobsearch::browser::host_of(u))
            .any(|h| Some(h) == host);
        let is_open = tabs_after
            .iter()
            .filter_map(|u| jobsearch::browser::host_of(u))
            .any(|h| Some(h) == host);
        match (was_open, is_open) {
            (true, _) => eprintln!("  {} - already open, skipping", url),
            (false, true) => eprintln!("  {} - opened", url),
            (false, false) => eprintln!("  {} - opened (loading...)", url),
        }
    }

    eprintln!("\nBrave is ready with {} tabs.", urls.len());
    eprintln!("Login to each site if needed, then run 'jobsearch update'.");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let db_path = std::env::var("DATABASE_URL")
        .ok()
        .and_then(|url| url.strip_prefix("sqlite:").map(std::path::PathBuf::from))
        .unwrap_or_else(|| {
            let dirs = ProjectDirs::from("", "", "jobsearch").expect("project dirs");
            dirs.data_dir().join("jobsearch.db")
        });

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = Db::open(&db_path).await?;
    let browser = BrowserManager::new();

    match cli.command {
        Commands::Init {} => {
            cmd_init(&browser, DEFAULT_INIT_URLS).await?;
        }
        Commands::Update(update_cmd) => match update_cmd.platform {
            UpdatePlatform::Upwork(args) => {
                let scraper =
                    UpworkScraper::with_config(args.tier, args.min_rate, args.client_hires);
                fetch_and_store(&db, &browser, &scraper, &args.query, args.pause).await?;
            }
            UpdatePlatform::Nofluff(args) => {
                let config = jobsearch::platforms::nofluffjobs::NoFluffJobsConfig {
                    path: "remote".to_string(),
                    min_salary_eur: args.min_salary,
                    employment: args.employment,
                    language: args.lang,
                    salary_currency: "EUR".to_string(),
                };
                let scraper = NoFluffJobsScraper::with_config(config);
                fetch_and_store(&db, &browser, &scraper, &args.query, args.pause).await?;
            }
            UpdatePlatform::Efinancialcareers(args) => {
                let config = EfinancialcareersConfig {
                    work_arrangement: "REMOTE".to_string(),
                    min_salary: args.min_salary,
                    currency_code: "USD".to_string(),
                    language: "en".to_string(),
                };
                let scraper = EfinancialcareersScraper::with_config(config);
                fetch_and_store(&db, &browser, &scraper, &args.query, args.pause_ms).await?;
            }
        },
        Commands::List(cmd) => match cmd.target {
            ListTarget::All(args) => {
                let filter = JobFilter {
                    recency: args.recency,
                    applied: args.applied,
                    liked: args.rating,
                };
                cmd_list(&db, None, filter, args.detailed, Sort::Created, cli.json).await?;
            }
            ListTarget::Upwork(args) => {
                let filter = JobFilter {
                    recency: args.common.recency,
                    applied: args.common.applied,
                    liked: args.common.rating,
                };
                let sort = match args.sort {
                    UpworkSortBy::Created => Sort::Created,
                    UpworkSortBy::UpworkViewed => Sort::UpworkViewed,
                };
                cmd_list(
                    &db,
                    Some(Platform::Upwork),
                    filter,
                    args.common.detailed,
                    sort,
                    cli.json,
                )
                .await?;
            }
            ListTarget::Nofluff(args) => {
                let filter = JobFilter {
                    recency: args.recency,
                    applied: args.applied,
                    liked: args.rating,
                };
                cmd_list(
                    &db,
                    Some(Platform::NoFluffJobs),
                    filter,
                    args.detailed,
                    Sort::Created,
                    cli.json,
                )
                .await?;
            }
            ListTarget::Efinancialcareers(args) => {
                let filter = JobFilter {
                    recency: args.recency,
                    applied: args.applied,
                    liked: args.rating,
                };
                cmd_list(
                    &db,
                    Some(Platform::Efinancialcareers),
                    filter,
                    args.detailed,
                    Sort::Created,
                    cli.json,
                )
                .await?;
            }
        },
        Commands::Show { id } => {
            cmd_show(&db, id, cli.json).await?;
        }
        Commands::Delete { ids } => {
            cmd_delete(&db, ids).await?;
        }
        Commands::React(cmd) => {
            cmd_react(&db, cmd.action).await?;
        }
        Commands::Serve { port } => {
            server::serve(db, port).await?;
        }
        Commands::Stats => {
            cmd_stats(&db, cli.json).await?;
        }
        Commands::Diagnose => {
            cmd_diagnose(&db, &db_path).await?;
        }
        Commands::SyncApplications(cmd) => match cmd.platform {
            SyncPlatform::Upwork(args) => {
                sync_apps(&UpworkScraper::new(), &browser, &db, args.pause_ms).await?;
            }
            SyncPlatform::Nofluff(args) => {
                sync_apps(&NoFluffJobsScraper::new(), &browser, &db, args.pause_ms).await?;
            }
            SyncPlatform::Efinancialcareers(args) => {
                sync_apps(
                    &EfinancialcareersScraper::new(),
                    &browser,
                    &db,
                    args.pause_ms,
                )
                .await?;
            }
        },
    }

    // Browser stays alive for reuse
    Ok(())
}

async fn sync_apps(
    client: &impl PlatformClient,
    browser: &BrowserManager,
    db: &Db,
    pause_ms: u64,
) -> Result<()> {
    let browser = browser.ensure().await?;
    match client.sync_applications(&browser, db, pause_ms, None).await {
        Ok(count) => eprintln!("Synced {} applications", count),
        Err(e) => eprintln!("Error syncing applications: {}", e),
    }
    Ok(())
}

async fn fetch_and_store(
    db: &Db,
    browser: &BrowserManager,
    client: &impl PlatformClient,
    query: &str,
    pause_ms: u64,
) -> Result<()> {
    eprintln!("Fetching from {}...", client.name());
    match client
        .fetch_with_manager(browser, db, query, pause_ms)
        .await
    {
        Ok(jobs) => {
            eprintln!("  Total new jobs: {}", jobs.len());
        }
        Err(e) => {
            eprintln!("  Error from {}: {}", client.name(), e);
        }
    }
    Ok(())
}

async fn cmd_list(
    db: &Db,
    platform: Option<Platform>,
    filter: JobFilter,
    detailed: bool,
    sort: Sort,
    json: bool,
) -> Result<()> {
    let jobs = db.list_jobs(platform, sort, i64::MAX).await?;
    let jobs = filter.apply(jobs);

    if json {
        println!("{}", serde_json::to_string_pretty(&jobs)?);
        return Ok(());
    }

    if detailed {
        for job in &jobs {
            println!("{}", display::render_job_detailed(job));
        }
        println!("\nTotal: {} jobs", jobs.len());
    } else {
        println!("{}", display::render_table(&jobs, platform));
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
        println!("{}", display::render_job_detailed(&job));
    }

    Ok(())
}

async fn cmd_delete(db: &Db, ids: Vec<i64>) -> Result<()> {
    let deleted = db.delete_jobs(&ids).await?;
    println!(
        "Deleted {} job{}",
        deleted,
        if deleted == 1 { "" } else { "s" }
    );
    Ok(())
}

async fn cmd_react(db: &Db, cmd: ReactAction) -> Result<()> {
    match cmd {
        ReactAction::Apply {
            id,
            note,
            note_file,
        } => {
            db.get_job(id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Job {} not found", id))?;
            let note = if let Some(n) = note {
                Some(n)
            } else if let Some(path) = note_file {
                Some(tokio::fs::read_to_string(shellexpand::path::tilde(&path)).await?)
            } else {
                None
            };
            db.set_applied(id, note.as_deref(), chrono::Utc::now())
                .await?;
            match note {
                Some(n) if !n.is_empty() => {
                    println!("Job {} marked applied with note:", id);
                    for line in n.lines() {
                        println!("  {}", line);
                    }
                }
                _ => println!("Job {} marked applied", id),
            }
        }
        ReactAction::Like { ids } => {
            db.set_liked(&ids, true).await?;
            println!(
                "Liked {} job{}",
                ids.len(),
                if ids.len() == 1 { "" } else { "s" }
            );
        }
        ReactAction::Dislike { ids } => {
            db.set_liked(&ids, false).await?;
            println!(
                "Disliked {} job{}",
                ids.len(),
                if ids.len() == 1 { "" } else { "s" }
            );
        }
        ReactAction::Neutral { ids } => {
            db.set_neutral(&ids).await?;
            println!(
                "Reset {} job{} to neutral",
                ids.len(),
                if ids.len() == 1 { "" } else { "s" }
            );
        }
    }
    Ok(())
}

async fn cmd_stats(db: &Db, json: bool) -> Result<()> {
    let stats = db.stats().await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&stats)?);
    } else {
        println!("Total jobs: {}", stats.total);
        println!("\nBy platform:");
        for (p, c) in &stats.by_platform {
            println!("  {}: {}", p, c);
        }
    }

    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["bytes", "KB", "MB", "GB", "TB"];
    if bytes == 0 {
        return "0 bytes".to_string();
    }
    let exp = (bytes as f64).log(1024.0).min(UNITS.len() as f64 - 1.0) as usize;
    let val = bytes as f64 / 1024f64.powi(exp as i32);
    if exp == 0 {
        format!("{} {}", bytes, UNITS[exp])
    } else {
        format!("{:.2} {}", val, UNITS[exp])
    }
}

async fn cmd_diagnose(db: &Db, db_path: &std::path::Path) -> Result<()> {
    let file_size = std::fs::metadata(db_path).ok().map(|m| m.len());
    let stats = db.stats().await?;

    let abs_path = db_path
        .canonicalize()
        .unwrap_or_else(|_| db_path.to_path_buf());

    println!("DB path: {}", abs_path.display());
    println!(
        "DB file size: {}",
        file_size.map_or("unknown".to_string(), format_bytes)
    );
    println!("Total jobs: {}", stats.total);
    println!("\nBy platform:");
    for (p, c) in &stats.by_platform {
        println!("  {}: {}", p, c);
    }

    Ok(())
}
