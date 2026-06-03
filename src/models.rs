use chrono::{DateTime, Utc};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Platform-specific scraped data stored on each job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "platform", rename_all = "lowercase")]
#[allow(clippy::large_enum_variant)]
pub enum Data {
    Upwork { detail: UpworkJobDetail },
    Nofluffjobs { detail: NoFluffJobDetail },
}

/// Job card as scraped from the Upwork list page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpworkJobCard {
    pub external_id: String,
    pub title: String,
    pub description: Option<String>,
    pub url: String,
    pub budget: Option<String>,
    pub posted_at_text: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Full detail scraped from an individual Upwork job page.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpworkJobDetail {
    #[serde(default)]
    pub proposals: String,
    #[serde(default)]
    pub last_viewed: String,
    #[serde(default)]
    pub interviewing: String,
    #[serde(default)]
    pub invites_sent: String,
    #[serde(default)]
    pub unanswered_invites: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub exact_budget: String,
    #[serde(default)]
    pub experience_level: String,
    #[serde(default)]
    pub hires: String,
    #[serde(default)]
    pub project_type: String,
    #[serde(default)]
    pub duration: String,
    #[serde(default)]
    pub hours_per_week: String,
}

/// Job card as scraped from the NoFluffJobs list page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoFluffJobCard {
    pub external_id: String,
    pub title: String,
    pub url: String,
    pub budget: Option<String>,
    pub posted_at_text: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Full detail scraped from an individual NoFluffJobs job page.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NoFluffJobDetail {
    #[serde(default)]
    pub company: String,
    #[serde(default)]
    pub seniority: String,
    #[serde(default)]
    pub remote: String,
    #[serde(default)]
    pub locations: Vec<String>,
    #[serde(default)]
    pub must_have: Vec<String>,
    #[serde(default)]
    pub requirements: String,
    #[serde(default)]
    pub offer_description: String,
    #[serde(default)]
    pub offer_valid_until: String,
}

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
    pub raw: Data,
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
