use crate::extractors::llm::DEFAULT_LLM_CLI;
use crate::platforms::upwork::UpworkTier;
use clap::{Args, Parser, Subcommand, ValueEnum};

pub const VERSION: &str = match option_env!("GIT_HASH") {
    Some(v) => v,
    None => "dev",
};

#[derive(Parser)]
#[command(name = "jobsearch", version = VERSION)]
#[command(about = "Unified job search CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Init,
    Update(UpdateCmd),
    List(ListCmd),
    Show(ShowCmd),
    Delete {
        ids: Vec<i64>,
    },
    React(ReactCmd),
    Serve {
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
    Diagnose,
    SyncApplications(SyncApplicationsCmd),
    Embed(EmbedCmd),
}

#[derive(Parser)]
pub struct EmbedCmd {
    /// Number of jobs to embed in one batch.
    #[arg(long, default_value = "16")]
    pub batch_size: usize,
}

#[derive(Parser)]
pub struct ListCmd {
    #[command(subcommand)]
    pub target: ListTarget,
}

#[derive(Subcommand)]
#[command(rename_all = "lower")]
pub enum ListTarget {
    All(AllListArgs),
    Upwork(UpworkListArgs),
    Nofluff(PlatformListArgs),
    Efinancialcareers(PlatformListArgs),
    Hackernews(PlatformListArgs),
    LinkedIn(PlatformListArgs),
}

#[derive(Args)]
pub struct AllListArgs {
    #[command(flatten)]
    pub common: CommonListArgs,

    /// Sort order: created, applied
    #[arg(long, value_enum, default_value = "created")]
    pub sort: CommonSortBy,
}

#[derive(Args)]
pub struct PlatformListArgs {
    #[command(flatten)]
    pub common: CommonListArgs,

    /// Sort order: created, applied
    #[arg(long, value_enum, default_value = "created")]
    pub sort: CommonSortBy,
}

#[derive(Args)]
pub struct ShowCmd {
    #[arg(required = true)]
    pub ids: Vec<i64>,
}

#[derive(Args)]
pub struct CommonListArgs {
    /// Filter by applied status: true/false. Omit for all.
    #[arg(long)]
    pub applied: Option<bool>,

    /// Filter by rating: liked, disliked, or neutral. Omit for all.
    #[arg(long)]
    pub rating: Option<crate::models::Rating>,

    /// Filter by remote status: true/false. Omit for all.
    #[arg(long)]
    pub remote: Option<bool>,

    /// Search jobs by semantic query text.
    #[arg(long)]
    pub search: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CommonSortBy {
    Created,
    Applied,
}

#[derive(Args)]
pub struct UpworkListArgs {
    #[command(flatten)]
    pub common: CommonListArgs,

    /// Sort order: created, `upwork_viewed`, applied
    #[arg(long, value_enum, default_value = "upwork_viewed")]
    pub sort: UpworkSortBy,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum UpworkSortBy {
    Created,
    #[value(name = "upwork_viewed")]
    UpworkViewed,
    Applied,
}

#[derive(Parser)]
pub struct UpdateCmd {
    #[command(subcommand)]
    pub platform: UpdatePlatform,
}

#[derive(Parser)]
pub struct SyncApplicationsCmd {
    #[command(subcommand)]
    pub platform: SyncPlatform,
}

#[derive(Subcommand)]
#[command(rename_all = "lower")]
pub enum SyncPlatform {
    Upwork(SyncArgs),
    Nofluff(SyncArgs),
    Efinancialcareers(SyncArgs),
}

#[derive(Args)]
pub struct SyncArgs {
    /// Pause between interactions in ms (default: 2000)
    #[arg(long, default_value = "2000")]
    pub pause_ms: u64,
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
#[command(rename_all = "lower")]
pub enum UpdatePlatform {
    /// Fetch Upwork jobs
    Upwork(UpworkArgs),

    /// Fetch `NoFluffJobs` jobs
    Nofluff(NofluffArgs),

    /// Fetch eFinancialCareers jobs
    Efinancialcareers(EfinancialcareersArgs),

    /// Fetch Hacker News "Who is hiring?" jobs
    Hackernews(HackernewsArgs),

    /// Fetch LinkedIn jobs
    LinkedIn(LinkedInArgs),
}

#[derive(Args)]
pub struct UpworkArgs {
    #[arg(short, long, default_value = "")]
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

#[derive(Args)]
pub struct EfinancialcareersArgs {
    /// Job title/keyword to search.
    #[arg(short, long, default_value = "")]
    pub query: String,

    /// Minimum annual salary in USD (default: 100000)
    #[arg(long, default_value = "100000")]
    pub min_salary: u32,

    /// Pause between interactions in ms (default: 2000)
    #[arg(long, default_value = "2000")]
    pub pause_ms: u64,
}

#[derive(Args)]
pub struct HackernewsArgs {
    /// Keyword search passed to Algolia (default: empty = all job posts).
    #[arg(short, long, default_value = "")]
    pub query: String,

    /// Candidate location used to interpret remote work offers (e.g. Europe or US).
    #[arg(long, default_value = "Europe")]
    pub location: String,

    /// LLM CLI command used to extract structured fields from HN comments.
    #[arg(long, default_value = DEFAULT_LLM_CLI)]
    pub llm_cli: String,
}

/// LinkedIn update arguments.
#[derive(Args)]
pub struct LinkedInArgs {
    /// Number of days back to search (default: 30)
    #[arg(long, default_value = "30")]
    pub since_days: u32,

    /// Pause between requests in ms
    #[arg(long, default_value_t = 2000)]
    pub pause_ms: u64,
}
