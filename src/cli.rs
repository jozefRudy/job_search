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
    Embed(EmbedCmd),
}

#[derive(Parser)]
pub struct EmbedCmd {
    /// Number of jobs to embed in one batch.
    #[arg(long, default_value = "16")]
    pub batch_size: usize,

    /// Reset all vectorized flags and re-embed every job.
    #[arg(long)]
    pub force: bool,
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
    Upwork,
    /// Fetch `NoFluffJobs` jobs
    Nofluff,
    /// Fetch eFinancialCareers jobs
    Efinancialcareers,
    /// Fetch Hacker News "Who is hiring?" jobs
    Hackernews,
    /// Fetch LinkedIn jobs
    LinkedIn,
}
