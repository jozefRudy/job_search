//! Application configuration loaded from `jobsearch.toml`.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub location: String,
    pub pause_ms: u64,
    pub providers: Providers,
    pub browser: BrowserConfig,
    pub llm: LlmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Command string for the LLM CLI, e.g. `pi --print --model ...`.
    pub cli: String,
}

/// Placeholder written into the sample config by `jobsearch init`.
pub const SAMPLE_BROWSER_BIN: &str = "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser";

/// Sample LLM CLI command written into the sample config by `jobsearch init`.
pub const SAMPLE_LLM_BIN: &str = "pi --print --no-session --no-tools --no-extensions --mode text --thinking off --model deepseek/deepseek-v4-flash";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    /// Path to a Chromium-based browser binary, launched with `--remote-debugging-port=9222`.
    pub bin: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Providers {
    pub upwork: ProviderConfig,
    pub nofluffjobs: ProviderConfig,
    pub efinancialcareers: ProviderConfig,
    pub linkedin: ProviderConfig,
    pub hackernews: ProviderConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    #[serde(default)]
    pub urls: Vec<String>,
    pub pause_ms: Option<u64>,
}

impl Settings {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config from {}", path.display()))?;
        let settings: Settings = toml::from_str(&content)
            .with_context(|| format!("failed to parse config from {}", path.display()))?;
        Ok(settings)
    }

    #[must_use]
    pub fn sample() -> Self {
        Self {
            location: "Europe".to_string(),
            pause_ms: 2000,
            browser: BrowserConfig {
                bin: SAMPLE_BROWSER_BIN.to_string(),
            },
            llm: LlmConfig {
                cli: SAMPLE_LLM_BIN.to_string(),
            },
            providers: Providers {
                upwork: ProviderConfig {
                    urls: vec!["https://www.upwork.com/nx/search/jobs/?q=trading&sort=recency&per_page=50&t=0&hourly_rate=60-".to_string()],
                    pause_ms: None,
                },
                nofluffjobs: ProviderConfig {
                    urls: vec!["https://nofluffjobs.com/remote?criteria=employment%3Db2b%20salary%3Eeur8000m%20jobLanguage%3Den&sort=newest".to_string()],
                    pause_ms: None,
                },
                efinancialcareers: ProviderConfig {
                    urls: vec!["https://www.efinancialcareers.com/jobs/remote/python?pageSize=50&filters.postedDate=SEVEN&language=en".to_string()],
                    pause_ms: None,
                },
                linkedin: ProviderConfig {
                    urls: vec!["https://www.linkedin.com/jobs/search/?f_I=4&f_T=9%2C25201%2C39&f_TPR=r200000&f_WT=2&geoId=92000000".to_string()],
                    pause_ms: None,
                },
                hackernews: ProviderConfig {
                    urls: vec!["https://hn.algolia.com/api/v1/search_by_date".to_string()],
                    pause_ms: None,
                },
            },
        }
    }

    #[must_use]
    pub fn provider_pause_ms(&self, name: &str) -> u64 {
        match name {
            "upwork" => self.providers.upwork.pause_ms.unwrap_or(self.pause_ms),
            "nofluffjobs" => self.providers.nofluffjobs.pause_ms.unwrap_or(self.pause_ms),
            "efinancialcareers" => self
                .providers
                .efinancialcareers
                .pause_ms
                .unwrap_or(self.pause_ms),
            "linkedin" => self.providers.linkedin.pause_ms.unwrap_or(self.pause_ms),
            "hackernews" => self.providers.hackernews.pause_ms.unwrap_or(self.pause_ms),
            _ => self.pause_ms,
        }
    }
}
