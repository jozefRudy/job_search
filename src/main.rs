use anyhow::{Result, bail};
use clap::Parser;
use directories::ProjectDirs;
use jobsearch::browser::{BrowserExt, BrowserManager, DEFAULT_INIT_URLS, ensure_init_tabs};
use jobsearch::cli::{
    Cli, Commands, CommonSortBy, ListTarget, ReactAction, UpdatePlatform, UpworkSortBy,
};
use jobsearch::config::Settings;
use jobsearch::db::Db;
use jobsearch::embed::{DEFAULT_EMBEDDING_MODEL, Embedder};
use jobsearch::embeddings_store::EmbeddingsStore;
use jobsearch::language::LanguageService;
use jobsearch::models::{JobFilter, Platform, Rating, Sort};
use jobsearch::platforms::{
    PlatformClient, efinancialcareers::EfinancialcareersScraper, hackernews::HackerNewsScraper,
    linkedin::LinkedInScraper, nofluffjobs::NoFluffJobsScraper, upwork::UpworkScraper,
};
use jobsearch::server;

fn config_path() -> std::path::PathBuf {
    match std::env::var_os("JOBSEARCH_CONFIG_DIR") {
        Some(dir) => {
            let dir = shellexpand::path::tilde(&dir).into_owned();
            dir.join("jobsearch.toml")
        }
        None => ProjectDirs::from("", "", "jobsearch")
            .expect("project dirs")
            .config_dir()
            .join("jobsearch.toml"),
    }
}

async fn cmd_init(manager: &BrowserManager, urls: &[&str]) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if !path.exists() {
        let sample = Settings::sample();
        let text = toml::to_string_pretty(&sample)?;
        std::fs::write(&path, text)?;
        eprintln!("Created sample config at {}", path.display());
    }

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

    let db_path = std::env::var("JOBSEARCH_DATABASE_URL")
        .ok()
        .and_then(|url| url.strip_prefix("sqlite:").map(std::path::PathBuf::from))
        .map_or_else(
            || {
                let dirs = ProjectDirs::from("", "", "jobsearch").expect("project dirs");
                dirs.data_dir().join("jobsearch.db")
            },
            |p| shellexpand::path::tilde(&p).into_owned(),
        );

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = Db::open(&db_path).await?;
    let browser = BrowserManager::new();

    match cli.command {
        Commands::Init => {
            cmd_init(&browser, DEFAULT_INIT_URLS).await?;
        }
        Commands::Update(update_cmd) => {
            let settings = Settings::load(&config_path())?;
            cmd_update(update_cmd, &db, &browser, &settings).await?;
        }
        Commands::List(cmd) => cmd_list_with_target(cmd, &db, &db_path).await?,
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
            server::serve(db, &db_path, port).await?;
        }
        Commands::Diagnose => {
            cmd_diagnose(&db, &db_path).await?;
        }
        Commands::Embed(cmd) => {
            cmd_embed(cmd, &db, &db_path).await?;
        }
    }

    Ok(())
}

async fn open_embeddings_store(db: &Db, db_path: &std::path::Path) -> Result<EmbeddingsStore> {
    let cache_dir = db_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let embedder = Embedder::load(cache_dir).await?;
    EmbeddingsStore::open(db_path, DEFAULT_EMBEDDING_MODEL, db.clone(), embedder).await
}

async fn cmd_embed(
    cmd: jobsearch::cli::EmbedCmd,
    db: &Db,
    db_path: &std::path::Path,
) -> Result<()> {
    let store = open_embeddings_store(db, db_path).await?;

    let _indexed = store
        .index_unvectorized(cmd.batch_size, |total| {
            eprint!("\r    Indexed {total:>5} jobs");
        })
        .await?;
    eprintln!();
    Ok(())
}

async fn cmd_update(
    update_cmd: jobsearch::cli::UpdateCmd,
    db: &Db,
    browser: &BrowserManager,
    settings: &Settings,
) -> Result<()> {
    let lang = LanguageService::new();
    match update_cmd.platform {
        UpdatePlatform::Upwork => {
            if settings.providers.upwork.urls.is_empty() {
                bail!("no URLs configured for upwork in jobsearch.toml");
            }
            let scraper = UpworkScraper::new();
            for url in &settings.providers.upwork.urls {
                fetch_and_store(
                    db,
                    browser,
                    &scraper,
                    url,
                    settings.provider_pause_ms("upwork"),
                )
                .await?;
            }
        }
        UpdatePlatform::Nofluff => {
            if settings.providers.nofluffjobs.urls.is_empty() {
                bail!("no URLs configured for nofluffjobs in jobsearch.toml");
            }
            let scraper = NoFluffJobsScraper::new(lang);
            for url in &settings.providers.nofluffjobs.urls {
                fetch_and_store(
                    db,
                    browser,
                    &scraper,
                    url,
                    settings.provider_pause_ms("nofluffjobs"),
                )
                .await?;
            }
        }
        UpdatePlatform::Efinancialcareers => {
            if settings.providers.efinancialcareers.urls.is_empty() {
                bail!("no URLs configured for efinancialcareers in jobsearch.toml");
            }
            let scraper = EfinancialcareersScraper::new(lang);
            for url in &settings.providers.efinancialcareers.urls {
                fetch_and_store(
                    db,
                    browser,
                    &scraper,
                    url,
                    settings.provider_pause_ms("efinancialcareers"),
                )
                .await?;
            }
        }
        UpdatePlatform::Hackernews => {
            if settings.providers.hackernews.urls.is_empty() {
                bail!("no URLs configured for hackernews in jobsearch.toml");
            }
            let llm_cli = std::env::var("LLM_CLI").ok();
            for url in &settings.providers.hackernews.urls {
                let scraper = HackerNewsScraper::new(llm_cli.clone(), &settings.location, url)?;
                fetch_and_store(
                    db,
                    browser,
                    &scraper,
                    url,
                    settings.provider_pause_ms("hackernews"),
                )
                .await?;
            }
        }
        UpdatePlatform::LinkedIn => {
            if settings.providers.linkedin.urls.is_empty() {
                bail!("no URLs configured for linkedin in jobsearch.toml");
            }
            for url in &settings.providers.linkedin.urls {
                let scraper = LinkedInScraper::new(url);
                fetch_and_store(
                    db,
                    browser,
                    &scraper,
                    url,
                    settings.provider_pause_ms("linkedin"),
                )
                .await?;
            }
        }
    }
    Ok(())
}

async fn cmd_list_with_target(
    cmd: jobsearch::cli::ListCmd,
    db: &Db,
    db_path: &std::path::Path,
) -> Result<()> {
    match cmd.target {
        ListTarget::All(args) => {
            let filter = JobFilter {
                platform: None,
                applied: args.common.applied,
                rating: args.common.rating,
                remote: args.common.remote,
            };
            let sort = match args.sort {
                CommonSortBy::Created => Sort::Created,
                CommonSortBy::Applied => Sort::Applied,
            };
            cmd_list(db, filter, sort, args.common.search, db_path).await?;
        }
        ListTarget::Upwork(args) => {
            let filter = JobFilter {
                platform: Some(Platform::Upwork),
                applied: args.common.applied,
                rating: args.common.rating,
                remote: args.common.remote,
            };
            let sort = match args.sort {
                UpworkSortBy::Created => Sort::Created,
                UpworkSortBy::UpworkViewed => Sort::UpworkViewed,
                UpworkSortBy::Applied => Sort::Applied,
            };
            cmd_list(db, filter, sort, args.common.search, db_path).await?;
        }
        ListTarget::Nofluff(args) => {
            let filter = JobFilter {
                platform: Some(Platform::NoFluffJobs),
                applied: args.common.applied,
                rating: args.common.rating,
                remote: args.common.remote,
            };
            let sort = match args.sort {
                CommonSortBy::Created => Sort::Created,
                CommonSortBy::Applied => Sort::Applied,
            };
            cmd_list(db, filter, sort, args.common.search, db_path).await?;
        }
        ListTarget::Efinancialcareers(args) => {
            let filter = JobFilter {
                platform: Some(Platform::Efinancialcareers),
                applied: args.common.applied,
                rating: args.common.rating,
                remote: args.common.remote,
            };
            let sort = match args.sort {
                CommonSortBy::Created => Sort::Created,
                CommonSortBy::Applied => Sort::Applied,
            };
            cmd_list(db, filter, sort, args.common.search, db_path).await?;
        }
        ListTarget::Hackernews(args) => {
            let filter = JobFilter {
                platform: Some(Platform::Hackernews),
                applied: args.common.applied,
                rating: args.common.rating,
                remote: args.common.remote,
            };
            let sort = match args.sort {
                CommonSortBy::Created => Sort::Created,
                CommonSortBy::Applied => Sort::Applied,
            };
            cmd_list(db, filter, sort, args.common.search, db_path).await?;
        }
        ListTarget::LinkedIn(args) => {
            let filter = JobFilter {
                platform: Some(Platform::LinkedIn),
                applied: args.common.applied,
                rating: args.common.rating,
                remote: args.common.remote,
            };
            let sort = match args.sort {
                CommonSortBy::Created => Sort::Created,
                CommonSortBy::Applied => Sort::Applied,
            };
            cmd_list(db, filter, sort, args.common.search, db_path).await?;
        }
    }
    Ok(())
}

async fn fetch_and_store(
    db: &Db,
    manager: &BrowserManager,
    client: &impl PlatformClient,
    url: &str,
    pause_ms: u64,
) -> Result<()> {
    eprintln!("Fetching from {}: {}...", client.name(), url);
    match client.fetch_with_manager(manager, db, url, pause_ms).await {
        Ok(state) => {
            eprintln!("    {}", state.summary());
        }
        Err(e) => {
            eprintln!("    Error from {}: {}", client.name(), e);
        }
    }
    Ok(())
}

async fn cmd_list(
    db: &Db,
    filter: JobFilter,
    sort: Sort,
    search: Option<String>,
    db_path: &std::path::Path,
) -> Result<()> {
    if let Some(query) = search.filter(|s| !s.is_empty()) {
        let store = open_embeddings_store(db, db_path).await?;
        let candidate_ids = db.filter_job_ids(&filter).await?;
        let query_embedding = store.embedder().embed_query(&query).await?;
        let ranked = store
            .search(&query_embedding, &candidate_ids, 1000, 0)
            .await?;
        let ids: Vec<i64> = ranked.into_iter().map(|(id, _)| id).collect();
        let jobs = db.get_jobs(&ids).await?;
        println!("{}", serde_json::to_string_pretty(&jobs)?);
        return Ok(());
    }

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

async fn dir_size(path: &std::path::Path) -> Result<u64> {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut entries = tokio::fs::read_dir(&dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let meta = entry.metadata().await?;
            if meta.is_file() {
                total += meta.len();
            } else if meta.is_dir() {
                stack.push(entry.path());
            }
        }
    }
    Ok(total)
}

async fn subdir_sizes(path: &std::path::Path) -> Result<Vec<(String, u64)>> {
    let mut result = Vec::new();
    let mut entries = tokio::fs::read_dir(path).await?;
    while let Some(entry) = entries.next_entry().await? {
        let meta = entry.metadata().await?;
        if meta.is_dir() {
            let full_path = entry.path();
            let size = dir_size(&full_path).await?;
            result.push((full_path.to_string_lossy().to_string(), size));
        }
    }
    Ok(result)
}

async fn cmd_diagnose(db: &Db, db_path: &std::path::Path) -> Result<()> {
    let file_size = std::fs::metadata(db_path).ok().map(|m| m.len());
    let stats = db.stats().await?;

    let abs_path = db_path
        .canonicalize()
        .unwrap_or_else(|_| db_path.to_path_buf());

    println!(
        "{}: {}",
        abs_path.display(),
        file_size.map_or("unknown".to_string(), format_bytes)
    );

    let base_dir = db_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));

    let models_dir = base_dir.join("models");
    if models_dir.exists() {
        for (name, size) in subdir_sizes(&models_dir).await? {
            println!("{name}: {}", format_bytes(size));
        }
    }

    let lance_dir = base_dir.join("lance");
    if lance_dir.exists() {
        for (name, size) in subdir_sizes(&lance_dir).await? {
            println!("{name}: {}", format_bytes(size));
        }
    }
    println!("Total jobs: {}", stats.total);

    Ok(())
}
