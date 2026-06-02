use chrono::{DateTime, Utc};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ValueEnum)]
#[clap(rename_all = "lower")]
#[sqlx(rename_all = "lowercase")]
pub enum Platform {
    NoFluffJobs,
    Upwork,
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobStatus::New => write!(f, "new"),
            JobStatus::Viewed => write!(f, "viewed"),
            JobStatus::Saved => write!(f, "saved"),
            JobStatus::Applied => write!(f, "applied"),
            JobStatus::Rejected => write!(f, "rejected"),
            JobStatus::Hidden => write!(f, "hidden"),
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Platform::NoFluffJobs => write!(f, "nofluffjobs"),
            Platform::Upwork => write!(f, "upwork"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Option<i64>,
    pub platform: Platform,
    pub external_id: String,
    pub title: String,
    pub description: Option<String>,
    pub url: String,
    pub posted_at: Option<DateTime<Utc>>,
    pub budget: Option<String>,
    pub tags: Vec<String>,
    pub raw: serde_json::Value,
    pub status: JobStatus,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ValueEnum)]
#[clap(rename_all = "lower")]
#[sqlx(rename_all = "lowercase")]
pub enum JobStatus {
    New,
    Viewed,
    Saved,
    Applied,
    Rejected,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ValueEnum)]
#[clap(rename_all = "lower")]
#[sqlx(rename_all = "lowercase")]
pub enum Reaction {
    Save,
    Apply,
    Hide,
}

impl fmt::Display for Reaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reaction::Save => write!(f, "save"),
            Reaction::Apply => write!(f, "apply"),
            Reaction::Hide => write!(f, "hide"),
        }
    }
}
