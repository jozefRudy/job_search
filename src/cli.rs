use crate::models::Recency;
use crate::platforms::upwork::UpworkTier;
use clap::{Args, Parser, Subcommand, ValueEnum};

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

    Serve {
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },

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
    /// Show platform-specific details below each row
    #[arg(long)]
    pub detailed: bool,

    /// Filter by recency, e.g. 1d, 4w
    #[arg(long)]
    pub recency: Option<Recency>,

    /// Filter by applied status: true/false. Omit for all.
    #[arg(long)]
    pub applied: Option<bool>,

    /// Filter by rating: liked, disliked, or neutral. Omit for all.
    #[arg(long)]
    pub rating: Option<crate::models::Rating>,
}

#[derive(Args)]
pub struct UpworkListArgs {
    #[command(flatten)]
    pub common: CommonListArgs,

    /// Sort order: created, upwork_viewed
    #[arg(long, value_enum, default_value = "upwork_viewed")]
    pub sort: UpworkSortBy,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum UpworkSortBy {
    Created,
    #[value(name = "upwork_viewed")]
    UpworkViewed,
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
    Apply {
        id: i64,
        /// Short single-line note
        #[arg(long, short, conflicts_with = "note_file")]
        note: Option<String>,
        /// Read note from file (for multiline cover letters)
        #[arg(long, short = 'f', conflicts_with = "note")]
        note_file: Option<std::path::PathBuf>,
    },

    /// Like one or more jobs
    Like { ids: Vec<i64> },

    /// Dislike one or more jobs
    Dislike { ids: Vec<i64> },

    /// Reset one or more jobs to neutral
    Neutral { ids: Vec<i64> },
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
