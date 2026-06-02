use crate::models::{JobStatus, Platform, Reaction};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "jobsearch")]
#[command(about = "Unified job search CLI")]
pub struct Cli {
    #[arg(short, long, global = true)]
    pub db: Option<PathBuf>,

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

    Update {
        #[arg(short, long)]
        platform: Option<Platform>,

        #[arg(short, long, default_value = "rust")]
        query: String,
    },

    List {
        #[arg(short, long)]
        platform: Option<Platform>,

        #[arg(short, long)]
        status: Option<JobStatus>,

        #[arg(short, long, default_value = "50")]
        limit: i64,
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
