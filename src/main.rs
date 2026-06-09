use anyhow::Result;
use clap::Parser;
use directories::ProjectDirs;
use jobsearch::browser::{BrowserExt, BrowserManager};
use jobsearch::cli::{Cli, Commands, ListTarget, ReactAction, UpdatePlatform, UpworkSortBy};
use jobsearch::db::Db;
use jobsearch::display;
use jobsearch::models::{JobFilter, Platform};
use jobsearch::platforms::{
    PlatformClient, nofluffjobs::NoFluffJobsScraper, upwork::UpworkScraper,
};
use jobsearch::server;

const DEFAULT_INIT_URLS: &[&str] = &[
    "https://www.upwork.com/freelancers/~01dba08086390dc196",
    "https://nofluffjobs.com",
];

async fn cmd_init(browser: &BrowserManager, urls: &[&str]) -> Result<()> {
    eprintln!("Launching Brave browser with {} tabs...", urls.len());

    let browser = browser.ensure().await?;
    let hosts = browser.get_page_hosts().await?;

    for url in urls.iter() {
        let host = jobsearch::browser::host_of(url);
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
                fetch_and_store(
                    &db,
                    &browser,
                    vec![Box::new(scraper)],
                    &args.query,
                    args.pause,
                )
                .await?;
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
                fetch_and_store(
                    &db,
                    &browser,
                    vec![Box::new(scraper)],
                    &args.query,
                    args.pause,
                )
                .await?;
            }
        },
        Commands::List(cmd) => match cmd.target {
            ListTarget::All(args) => {
                let filter = JobFilter {
                    recency: args.recency,
                    applied: args.applied,
                    liked: args.rating,
                };
                cmd_list(
                    &db,
                    None,
                    filter,
                    args.detailed,
                    |a, b| b.created_at.cmp(&a.created_at),
                    cli.json,
                )
                .await?;
            }
            ListTarget::Upwork(args) => {
                let filter = JobFilter {
                    recency: args.common.recency,
                    applied: args.common.applied,
                    liked: args.common.rating,
                };
                match args.sort {
                    UpworkSortBy::Created => {
                        cmd_list(
                            &db,
                            Some(Platform::Upwork),
                            filter,
                            args.common.detailed,
                            |a, b| b.created_at.cmp(&a.created_at),
                            cli.json,
                        )
                        .await?;
                    }
                    UpworkSortBy::Viewed => {
                        cmd_list(
                            &db,
                            Some(Platform::Upwork),
                            filter,
                            args.common.detailed,
                            |a, b| {
                                use jobsearch::models::Data;
                                let Data::Upwork { detail: ad } = &a.raw else {
                                    unreachable!("upwork sort only for upwork jobs")
                                };
                                let Data::Upwork { detail: bd } = &b.raw else {
                                    unreachable!("upwork sort only for upwork jobs")
                                };
                                let av = ad.last_viewed;
                                let bv = bd.last_viewed;
                                (bv.is_some(), bv).cmp(&(av.is_some(), av))
                            },
                            cli.json,
                        )
                        .await?;
                    }
                }
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
                    |a, b| b.created_at.cmp(&a.created_at),
                    cli.json,
                )
                .await?;
            }
        },
        Commands::Show { id } => {
            cmd_show(&db, id, cli.json).await?;
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
    }

    // Browser stays alive for reuse
    Ok(())
}

async fn fetch_and_store(
    db: &Db,
    browser: &BrowserManager,
    clients: Vec<Box<dyn PlatformClient>>,
    query: &str,
    pause_ms: u64,
) -> Result<()> {
    for client in clients {
        eprintln!("Fetching from {}...", client.name());
        match client
            .fetch_with_manager(browser, db, query, pause_ms)
            .await
        {
            Ok(_jobs) => {}
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
    filter: JobFilter,
    detailed: bool,
    mut sort: impl FnMut(&jobsearch::models::Job, &jobsearch::models::Job) -> std::cmp::Ordering,
    json: bool,
) -> Result<()> {
    let jobs = db.list_jobs(platform, i64::MAX).await?;
    let mut jobs = filter.apply(jobs);

    jobs.sort_by(&mut sort);

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
            db.set_applied(id, note.as_deref()).await?;
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
