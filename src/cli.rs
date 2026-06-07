use crate::platforms::upwork::UpworkTier;
use clap::{Args, Parser, Subcommand, ValueEnum};

/// Parsed recency like "1d" or "4w". Stores days.
#[derive(Debug, Clone)]
pub struct Recency(pub i64);

impl std::str::FromStr for Recency {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.len() < 2 {
            anyhow::bail!("recency must be like 1d or 4w, got '{}'", s);
        }
        let (num, unit) = s.split_at(s.len() - 1);
        let n: i64 = num
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid recency number '{}'", num))?;
        let days = match unit {
            "d" => n,
            "w" => n * 7,
            _ => anyhow::bail!("recency unit must be 'd' or 'w', got '{}'", unit),
        };
        Ok(Recency(days))
    }
}

#[derive(Parser)]
#[command(name = "jobsearch")]
#[command(about = "Unified job search CLI")]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize browser session (opens Brave with required tabs)
    Init {},

    /// Fetch fresh jobs from platforms
    Update(UpdateCmd),

    List(ListCmd),

    Show {
        id: i64,
    },

    React(ReactCmd),

    Stats,

    /// Show diagnostic info (DB path, job count, env)
    Diagnose,
}

#[derive(Parser)]
pub struct ListCmd {
    #[command(subcommand)]
    pub target: ListTarget,
}

#[derive(Subcommand)]
pub enum ListTarget {
    All(CommonListArgs),
    Upwork(UpworkListArgs),
    Nofluff(CommonListArgs),
}

#[derive(Args)]
pub struct CommonListArgs {
    #[arg(short, long)]
    pub limit: Option<i64>,

    /// Show platform-specific details below each row
    #[arg(long)]
    pub detailed: bool,

    /// Filter by recency, e.g. 1d, 4w
    #[arg(long)]
    pub recency: Option<Recency>,

    /// Filter by applied status: true/false. Omit for all.
    #[arg(long)]
    pub applied: Option<bool>,

    /// Filter by liked status: true/false. Omit for all.
    #[arg(long)]
    pub liked: Option<bool>,
}

#[derive(Args)]
pub struct UpworkListArgs {
    #[command(flatten)]
    pub common: CommonListArgs,

    /// Sort order: created, viewed
    #[arg(long, value_enum, default_value = "viewed")]
    pub sort: UpworkSortBy,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum UpworkSortBy {
    Created,
    Viewed,
}

#[derive(Parser)]
pub struct UpdateCmd {
    #[command(subcommand)]
    pub platform: UpdatePlatform,
}

#[derive(Parser)]
pub struct ReactCmd {
    #[command(subcommand)]
    pub action: ReactAction,
}

#[derive(Subcommand)]
pub enum ReactAction {
    /// Apply to a job (optional note)
    Apply { id: i64, note: Option<String> },

    /// Like one or more jobs
    Like { ids: Vec<i64> },

    /// Unlike one or more jobs
    Unlike { ids: Vec<i64> },
}

#[derive(Subcommand)]
pub enum UpdatePlatform {
    /// Fetch Upwork jobs
    Upwork(UpworkArgs),

    /// Fetch NoFluffJobs jobs
    Nofluff(NofluffArgs),
}

#[derive(Args)]
pub struct UpworkArgs {
    #[arg(short, long, default_value = "rust")]
    pub query: String,

    /// Tier filter: expert, intermediate, both-upper (default: all tiers)
    #[arg(long, value_enum)]
    pub tier: Option<UpworkTier>,

    /// Minimum hourly rate in USD (default: no minimum)
    #[arg(long)]
    pub min_rate: Option<u32>,

    /// Client hire history filter, e.g. "1-9,10-"
    #[arg(long)]
    pub client_hires: Option<String>,

    /// Pause between interactions in ms (default: 2000)
    #[arg(long, default_value = "2000")]
    pub pause: u64,
}

#[derive(Args)]
pub struct NofluffArgs {
    #[arg(short, long, default_value = "")]
    pub query: String,

    /// Minimum monthly salary in EUR (default: no minimum)
    #[arg(long)]
    pub min_salary: Option<u32>,

    /// Employment type: b2b, permanent, contract (default: all)
    #[arg(long)]
    pub employment: Option<String>,

    /// Job language: en, pl, etc. (default: all)
    #[arg(long)]
    pub lang: Option<String>,

    /// Pause between interactions in ms (default: 2000)
    #[arg(long, default_value = "2000")]
    pub pause: u64,
}
