use crate::models::{Platform, Reaction};
use crate::platforms::upwork::UpworkTier;
use clap::{Args, Parser, Subcommand};

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
    Init {
        /// Platform-specific URLs to open (default: upwork, nofluffjobs)
        #[arg(short, long)]
        urls: Option<Vec<String>>,
    },

    /// Fetch fresh jobs from platforms
    Update(UpdateCmd),

    List {
        #[arg(short, long)]
        platform: Option<Platform>,

        #[arg(short, long)]
        limit: Option<i64>,

        /// Show only this job ID
        #[arg(long)]
        id: Option<i64>,

        /// Show platform-specific details below each row
        #[arg(long)]
        detailed: bool,
    },

    Show {
        id: i64,
    },

    React {
        id: i64,
        action: Reaction,
    },

    Stats,

    Detail {
        id: i64,

        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Parser)]
pub struct UpdateCmd {
    #[command(subcommand)]
    pub platform: UpdatePlatform,
}

#[derive(Subcommand)]
pub enum UpdatePlatform {
    /// Fetch Upwork jobs
    Upwork(UpworkArgs),

    /// Fetch NoFluffJobs jobs
    Nofluff(NofluffArgs),

    /// Fetch from all platforms
    All(AllArgs),
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
pub struct AllArgs {
    #[arg(short, long, default_value = "rust")]
    pub query: String,

    #[arg(long, value_enum)]
    pub upwork_tier: Option<UpworkTier>,

    #[arg(long)]
    pub upwork_min_rate: Option<u32>,

    #[arg(long)]
    pub nofluff_min_salary: Option<u32>,

    #[arg(long)]
    pub nofluff_employment: Option<String>,

    #[arg(long)]
    pub nofluff_lang: Option<String>,

    /// Pause between interactions in ms (default: 2000)
    #[arg(long, default_value = "2000")]
    pub pause: u64,
}
