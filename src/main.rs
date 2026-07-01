use anyhow::{Context, Result, bail};
use clap::Parser;
use directories::ProjectDirs;
use jobsearch::browser::{BrowserExt, BrowserManager, DEFAULT_INIT_URLS, ensure_init_tabs};
use jobsearch::cli::{
    Cli, Commands, CommonSortBy, ListTarget, ReactAction, SyncPlatform, UpdatePlatform,
    UpworkSortBy,
};
use jobsearch::db::Db;
use jobsearch::language::LanguageService;
use jobsearch::models::{JobFilter, Platform, Rating, Sort};
use jobsearch::platforms::{
    PlatformClient,
    efinancialcareers::{EfinancialcareersConfig, EfinancialcareersScraper},
    hackernews::{HackerNewsConfig, HackerNewsScraper},
    linkedin::LinkedInScraper,
    nofluffjobs::NoFluffJobsScraper,
    upwork::UpworkScraper,
};
use jobsearch::server;

async fn cmd_init(manager: &BrowserManager, urls: &[&str]) -> Result<()> {
    eprintln!("Launching Brave browser with {} tabs...", urls.len());

    let browser = manager.browser().await?;
    let tabs_before = browser.get_page_urls().await?;
    ensure_init_tabs(&browser, urls).await?;
    let tabs_after = browser.get_page_urls().await?;

    for url in urls {
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
            (true, _) => eprintln!("  {url} - already open, skipping"),
            (false, true) => eprintln!("  {url} - opened"),
            (false, false) => eprintln!("  {url} - opened (loading...)"),
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
        Commands::Init => {
            cmd_init(&browser, DEFAULT_INIT_URLS).await?;
        }
        Commands::Update(update_cmd) => cmd_update(update_cmd, &db, &browser).await?,
        Commands::List(cmd) => cmd_list_with_target(cmd, &db).await?,
        Commands::Show(args) => {
            let jobs = db.get_jobs(&args.ids).await?;
            println!("{}", serde_json::to_string_pretty(&jobs)?);
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
        Commands::Diagnose => {
            cmd_diagnose(&db, &db_path).await?;
        }
        Commands::SyncApplications(cmd) => cmd_sync_applications(cmd, &db, &browser).await?,
        Commands::SyncLikes { from, to } => {
            cmd_sync_likes(&from, &to).await?;
        }
    }

    // Browser stays alive for reuse
    Ok(())
}

async fn cmd_update(
    update_cmd: jobsearch::cli::UpdateCmd,
    db: &Db,
    browser: &BrowserManager,
) -> Result<()> {
    match update_cmd.platform {
        UpdatePlatform::Upwork(args) => {
            let scraper = UpworkScraper::with_config(args.tier, args.min_rate, args.client_hires);
            fetch_and_store(db, browser, &scraper, &args.query, args.pause).await?;
        }
        UpdatePlatform::Nofluff(args) => {
            let lang = LanguageService::new();
            let config = jobsearch::platforms::nofluffjobs::NoFluffJobsConfig {
                path: "remote".to_string(),
                min_salary_eur: args.min_salary,
                employment: args.employment,
                language: args.lang,
                salary_currency: "EUR".to_string(),
            };
            let scraper = NoFluffJobsScraper::with_config(config, lang);
            fetch_and_store(db, browser, &scraper, &args.query, args.pause).await?;
        }
        UpdatePlatform::Efinancialcareers(args) => {
            let lang = LanguageService::new();
            let config = EfinancialcareersConfig {
                work_arrangement: "REMOTE".to_string(),
                min_salary: args.min_salary,
                currency_code: "USD".to_string(),
                language: "en".to_string(),
            };
            let scraper = EfinancialcareersScraper::with_config(config, lang);
            fetch_and_store(db, browser, &scraper, &args.query, args.pause_ms).await?;
        }
        UpdatePlatform::Hackernews(args) => {
            let config = HackerNewsConfig {
                location: args.location,
            };
            let scraper = HackerNewsScraper::new(Some(args.llm_cli), &config);
            fetch_and_store(db, browser, &scraper, &args.query, 0).await?;
        }
        UpdatePlatform::LinkedIn(args) => {
            let scraper = LinkedInScraper::new(args.since_days);
            fetch_and_store(db, browser, &scraper, "", args.pause_ms).await?;
        }
    }
    Ok(())
}

async fn cmd_list_with_target(cmd: jobsearch::cli::ListCmd, db: &Db) -> Result<()> {
    match cmd.target {
        ListTarget::All(args) => {
            let filter = JobFilter {
                platform: None,
                applied: args.common.applied,
                rating: args.common.rating,
                remote: args.common.remote,
                is_english: args.common.english,
            };
            let sort = match args.sort {
                CommonSortBy::Created => Sort::Created,
                CommonSortBy::Applied => Sort::Applied,
            };
            cmd_list(db, filter, sort).await?;
        }
        ListTarget::Upwork(args) => {
            let filter = JobFilter {
                platform: Some(Platform::Upwork),
                applied: args.common.applied,
                rating: args.common.rating,
                remote: args.common.remote,
                is_english: args.common.english,
            };
            let sort = match args.sort {
                UpworkSortBy::Created => Sort::Created,
                UpworkSortBy::UpworkViewed => Sort::UpworkViewed,
                UpworkSortBy::Applied => Sort::Applied,
            };
            cmd_list(db, filter, sort).await?;
        }
        ListTarget::Nofluff(args) => {
            let filter = JobFilter {
                platform: Some(Platform::NoFluffJobs),
                applied: args.common.applied,
                rating: args.common.rating,
                remote: args.common.remote,
                is_english: args.common.english,
            };
            let sort = match args.sort {
                CommonSortBy::Created => Sort::Created,
                CommonSortBy::Applied => Sort::Applied,
            };
            cmd_list(db, filter, sort).await?;
        }
        ListTarget::Efinancialcareers(args) => {
            let filter = JobFilter {
                platform: Some(Platform::Efinancialcareers),
                applied: args.common.applied,
                rating: args.common.rating,
                remote: args.common.remote,
                is_english: args.common.english,
            };
            let sort = match args.sort {
                CommonSortBy::Created => Sort::Created,
                CommonSortBy::Applied => Sort::Applied,
            };
            cmd_list(db, filter, sort).await?;
        }
        ListTarget::Hackernews(args) => {
            let filter = JobFilter {
                platform: Some(Platform::Hackernews),
                applied: args.common.applied,
                rating: args.common.rating,
                remote: args.common.remote,
                is_english: args.common.english,
            };
            let sort = match args.sort {
                CommonSortBy::Created => Sort::Created,
                CommonSortBy::Applied => Sort::Applied,
            };
            cmd_list(db, filter, sort).await?;
        }
        ListTarget::LinkedIn(args) => {
            let filter = JobFilter {
                platform: Some(Platform::LinkedIn),
                applied: args.common.applied,
                rating: args.common.rating,
                remote: args.common.remote,
                is_english: args.common.english,
            };
            let sort = match args.sort {
                CommonSortBy::Created => Sort::Created,
                CommonSortBy::Applied => Sort::Applied,
            };
            cmd_list(db, filter, sort).await?;
        }
    }
    Ok(())
}

async fn cmd_sync_applications(
    cmd: jobsearch::cli::SyncApplicationsCmd,
    db: &Db,
    browser: &BrowserManager,
) -> Result<()> {
    match cmd.platform {
        SyncPlatform::Upwork(args) => {
            sync_apps(&UpworkScraper::new(), browser, db, args.pause_ms).await?;
        }
        SyncPlatform::Nofluff(args) => {
            let lang = LanguageService::new();
            sync_apps(&NoFluffJobsScraper::new(lang), browser, db, args.pause_ms).await?;
        }
        SyncPlatform::Efinancialcareers(args) => {
            let lang = LanguageService::new();
            sync_apps(
                &EfinancialcareersScraper::new(lang),
                browser,
                db,
                args.pause_ms,
            )
            .await?;
        }
    }
    Ok(())
}

async fn cmd_sync_likes(from: &std::path::Path, to: &std::path::Path) -> Result<()> {
    if !from.exists() {
        bail!("source file does not exist: {}", from.display());
    }
    if !to.exists() {
        bail!("target file does not exist: {}", to.display());
    }
    let target = Db::open(to).await?;
    let synced = target
        .sync_likes(from.to_str().context("invalid source path")?)
        .await?;
    println!(
        "Synced {} like{}",
        synced,
        if synced == 1 { "" } else { "s" }
    );
    Ok(())
}

async fn sync_apps(
    client: &impl PlatformClient,
    manager: &BrowserManager,
    db: &Db,
    pause_ms: u64,
) -> Result<()> {
    let browser = manager.browser().await?;
    eprintln!("Syncing applications from {}...", client.name());
    match client.sync_applications(&browser, db, pause_ms, None).await {
        Ok(state) => {
            eprintln!("    {}", state.summary());
        }
        Err(e) => {
            eprintln!();
            eprintln!(
                "\r    Error syncing applications from {}: {}",
                client.name(),
                e
            );
        }
    }
    Ok(())
}

async fn fetch_and_store(
    db: &Db,
    manager: &BrowserManager,
    client: &impl PlatformClient,
    query: &str,
    pause_ms: u64,
) -> Result<()> {
    eprintln!("Fetching from {}...", client.name());
    match client
        .fetch_with_manager(manager, db, query, pause_ms)
        .await
    {
        Ok(state) => {
            eprintln!("    {}", state.summary());
        }
        Err(e) => {
            eprintln!("    Error from {}: {}", client.name(), e);
        }
    }
    Ok(())
}

async fn cmd_list(db: &Db, filter: JobFilter, sort: Sort) -> Result<()> {
    let jobs = db.list_jobs_filtered(&filter, sort, i64::MAX, 0).await?;
    println!("{}", serde_json::to_string_pretty(&jobs.items)?);
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
                .ok_or_else(|| anyhow::anyhow!("Job {id} not found"))?;
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
                    println!("Job {id} marked applied with note:");
                    for line in n.lines() {
                        println!("  {line}");
                    }
                }
                _ => println!("Job {id} marked applied"),
            }
        }
        ReactAction::Like { ids } => {
            db.set_rating(&ids, Rating::Liked).await?;
            println!(
                "Liked {} job{}",
                ids.len(),
                if ids.len() == 1 { "" } else { "s" }
            );
        }
        ReactAction::Dislike { ids } => {
            db.set_rating(&ids, Rating::Disliked).await?;
            println!(
                "Disliked {} job{}",
                ids.len(),
                if ids.len() == 1 { "" } else { "s" }
            );
        }
        ReactAction::Neutral { ids } => {
            db.set_rating(&ids, Rating::Neutral).await?;
            println!(
                "Reset {} job{} to neutral",
                ids.len(),
                if ids.len() == 1 { "" } else { "s" }
            );
        }
    }
    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["bytes", "KB", "MB", "GB", "TB"];
    if bytes == 0 {
        return "0 bytes".to_string();
    }
    let exp = log_base(bytes, 1024).min(f64::from(
        u32::try_from(UNITS.len())
            .unwrap_or(u32::MAX)
            .saturating_sub(1),
    )) as usize;
    let val = bytes as f64 / 1024f64.powi(exp.try_into().unwrap_or(i32::MAX));
    if exp == 0 {
        format!("{} {}", bytes, UNITS[exp])
    } else {
        format!("{:.2} {}", val, UNITS[exp])
    }
}

fn log_base(n: u64, base: u32) -> f64 {
    (n as f64).log(f64::from(base))
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
        println!("  {p}: {c}");
    }

    Ok(())
}
