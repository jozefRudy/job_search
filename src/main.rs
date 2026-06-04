use anyhow::Result;
use clap::Parser;
use directories::ProjectDirs;
use jobsearch::browser::{BrowserExt, BrowserManager};
use jobsearch::cli::{Cli, Commands, UpdatePlatform};
use jobsearch::db::Db;
use jobsearch::display;
use jobsearch::models::{Data, Platform, Reaction};
use jobsearch::platforms::{
    PlatformClient, nofluffjobs::NoFluffJobsScraper, upwork::UpworkScraper,
};

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
        Commands::Init { urls } => {
            let init_urls: Vec<String> = match urls {
                Some(u) => u.clone(),
                None => DEFAULT_INIT_URLS.iter().map(|s| s.to_string()).collect(),
            };
            let init_urls: Vec<&str> = init_urls.iter().map(|s| s.as_str()).collect();
            cmd_init(&browser, &init_urls).await?;
        }
        Commands::Update(update_cmd) => match update_cmd.platform {
            UpdatePlatform::Upwork(args) => {
                let scraper = UpworkScraper::with_config(args.tier, args.min_rate);
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
            UpdatePlatform::All(args) => {
                let nf_config = jobsearch::platforms::nofluffjobs::NoFluffJobsConfig {
                    path: "remote".to_string(),
                    min_salary_eur: args.nofluff_min_salary,
                    employment: args.nofluff_employment,
                    language: args.nofluff_lang,
                    salary_currency: "EUR".to_string(),
                };
                let clients: Vec<Box<dyn PlatformClient>> = vec![
                    Box::new(NoFluffJobsScraper::with_config(nf_config)),
                    Box::new(UpworkScraper::with_config(
                        args.upwork_tier,
                        args.upwork_min_rate,
                    )),
                ];
                fetch_and_store(&db, &browser, clients, &args.query, args.pause).await?;
            }
        },
        Commands::List {
            platform,
            limit,
            id,
            detailed,
        } => {
            if let Some(job_id) = id {
                let job = db
                    .get_job(job_id)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("Job {} not found", job_id))?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&job)?);
                } else if detailed {
                    println!("{}", display::render_job_detailed(&job));
                } else {
                    println!("{}", display::render_table(&[job]));
                }
            } else {
                cmd_list(&db, platform, limit, detailed, cli.json).await?;
            }
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
            Ok(jobs) => {
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
    limit: Option<i64>,
    detailed: bool,
    json: bool,
) -> Result<()> {
    let jobs = db.list_jobs(platform, limit.unwrap_or(i64::MAX)).await?;

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
        println!("{}", display::render_table(&jobs));
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
        println!("Title:     {}", job.title);
        println!("URL:       {}", job.url);
        println!(
            "Posted:    {}",
            job.created_at
                .map(|d| d.to_rfc3339())
                .unwrap_or_else(|| "?".to_string())
        );
        println!("Budget:    {}", job.budget.as_deref().unwrap_or("?"));
        println!("Tags:      {}", job.tags.join(", "));
        println!("Desc:      {}", job.description.as_deref().unwrap_or("?"));

        // Show cached detail
        match &job.raw {
            Data::Upwork { detail } => {
                println!("\n--- Detail ---");
                println!("Exact budget:   {}", detail.exact_budget);
                println!("Experience:     {}", detail.experience_level);
                println!("Project type:   {}", detail.project_type);
                println!("Duration:       {}", detail.duration);
                println!("Hours/week:     {}", detail.hours_per_week);
                println!("Hires:          {}", detail.hires);
                println!("Proposals:      {}", detail.proposals);
                println!("Last viewed:    {}", detail.last_viewed);
                println!("Interviewing:   {}", detail.interviewing);
                println!("Invites sent:   {}", detail.invites_sent);
                println!("Unanswered:     {}", detail.unanswered_invites);
            }
            Data::Nofluffjobs { detail } => {
                println!("\n--- Detail ---");
                println!("Company:        {}", detail.company);
                println!("Seniority:      {}", detail.seniority);
                println!("Remote:         {}", detail.remote);
                println!("Locations:      {}", detail.locations.join(", "));
                println!("Valid until:    {}", detail.offer_valid_until);
                println!("Must have:      {}", detail.must_have.join(", "));
            }
        }
    }

    Ok(())
}

async fn cmd_react(db: &Db, id: i64, action: Reaction) -> Result<()> {
    db.add_reaction(id, action, None).await?;
    println!("Job {} reacted: {:?}", id, action);
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

async fn cmd_detail(db: &Db, browser: &BrowserManager, id: i64, force: bool) -> Result<()> {
    let job = db
        .get_job(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Job {} not found", id))?;

    match job.platform {
        Platform::Upwork => {
            let is_fresh = job.created_at.is_some_and(|created| {
                let age = chrono::Utc::now() - created;
                age.num_days() < 7
            });
            let should_fetch = force || is_fresh;

            let detail = if should_fetch {
                eprintln!("Fetching fresh detail...");
                let b = browser.ensure().await?;
                let scraper = UpworkScraper::new();
                let d = scraper.fetch_job_detail(&b, &job.url).await?;

                let raw = Data::Upwork { detail: d.clone() };
                db.update_raw(id, &raw).await?;

                d
            } else {
                eprintln!("Using cached detail (use --force to refetch)");
                match &job.raw {
                    Data::Upwork { detail } => detail.clone(),
                    _ => return Err(anyhow::anyhow!("Not an Upwork job")),
                }
            };

            println!("Title:          {}", job.title);
            println!("Budget:         {}", detail.exact_budget);
            println!("Experience:     {}", detail.experience_level);
            println!("Project type:   {}", detail.project_type);
            println!("Duration:       {}", detail.duration);
            println!("Hours/week:     {}", detail.hours_per_week);
            println!("Hires:          {}", detail.hires);
            println!("Proposals:      {}", detail.proposals);
            println!("Last viewed:    {}", detail.last_viewed);
            println!("Interviewing:   {}", detail.interviewing);
            println!("Invites sent:   {}", detail.invites_sent);
            println!("Unanswered:     {}", detail.unanswered_invites);
            println!("\nDescription:\n{}", detail.description);
        }
        Platform::NoFluffJobs => {
            let should_fetch = force;

            let detail = if should_fetch {
                eprintln!("Fetching fresh detail...");
                let scraper = NoFluffJobsScraper::new();
                let job_id = job.external_id.clone();
                let d = scraper.fetch_detail(&job_id).await?;

                let raw = Data::Nofluffjobs { detail: d.clone() };
                db.update_raw(id, &raw).await?;

                d
            } else {
                eprintln!("Using cached detail (use --force to refetch)");
                match &job.raw {
                    Data::Nofluffjobs { detail } => detail.clone(),
                    _ => return Err(anyhow::anyhow!("Not a NoFluffJobs job")),
                }
            };

            println!("Title:          {}", job.title);
            println!("Company:        {}", detail.company);
            println!("Seniority:      {}", detail.seniority);
            println!("Remote:         {}", detail.remote);
            println!("Locations:      {}", detail.locations.join(", "));
            println!("Valid until:    {}", detail.offer_valid_until);
            println!("Must have:      {}", detail.must_have.join(", "));
            println!("\nRequirements:\n{}", detail.requirements);
            println!("\nOffer description:\n{}", detail.offer_description);
        }
    }

    Ok(())
}
