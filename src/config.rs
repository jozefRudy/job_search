//! Application configuration loaded from `jobsearch.toml`.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Expand a leading `~` in a path; relative paths are returned unchanged.
#[must_use]
pub fn expand_tilde(path: impl AsRef<Path>) -> PathBuf {
    shellexpand::path::tilde(path.as_ref()).into_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub personal_info: PersonalInfo,
    pub pause_ms: u64,
    pub providers: Providers,
    pub browser: BrowserConfig,
    pub llm: LlmConfig,
    pub systemone: SystemOneConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// LLM CLI binary path.
    pub bin: String,
    /// LLM CLI args, whitespace-separated (split once in [`shared_llm`]).
    #[serde(default)]
    pub args: String,
}

/// Build the shared LLM handle from config; one per process (shared
/// concurrency cap across scrapers).
#[must_use]
pub fn shared_llm(cfg: &LlmConfig) -> patterns::llm_cli::SharedLlm {
    patterns::llm_cli::SharedLlm::new(
        cfg.bin.clone(),
        cfg.args
            .split_whitespace()
            .map(String::from)
            .collect::<Vec<String>>(),
        patterns::llm_cli::ConcurrencyLimits::default(),
    )
}

/// Placeholder written into the sample config by `jobsearch init`.
pub const SAMPLE_BROWSER_BIN: &str = "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser";

/// Sample LLM CLI config written into the sample config by `jobsearch init`.
pub const SAMPLE_LLM_BIN: &str = "pi";

/// Sample LLM CLI args written into the sample config by `jobsearch init`.
pub const SAMPLE_LLM_ARGS: &str = "--print --no-session --no-tools --no-extensions --mode text --thinking off --model deepseek/deepseek-flash";

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
    pub workatastartup: ProviderConfig,
    pub wellfound: ProviderConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    #[serde(default)]
    pub urls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalInfo {
    /// User region, shared by scrapers and prompt contexts.
    pub location: crate::region::Region,
    /// Path to the CV file: absolute, `~`-prefixed, or relative to the current working directory.
    pub cv: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemOneConfig {
    /// SystemOne base URL; omit to use [`DEFAULT_TYPESAFE_BASE_URL`].
    #[serde(default = "default_typesafe_base_url")]
    pub typesafe_base_url: String,
    /// SystemOne model; omit to use [`DEFAULT_TYPESAFE_MODEL`].
    #[serde(default = "default_typesafe_default_model")]
    pub typesafe_default_model: String,
}

fn default_typesafe_base_url() -> String {
    DEFAULT_TYPESAFE_BASE_URL.to_owned()
}

fn default_typesafe_default_model() -> String {
    DEFAULT_TYPESAFE_MODEL.to_owned()
}

impl Default for SystemOneConfig {
    fn default() -> Self {
        Self {
            typesafe_base_url: default_typesafe_base_url(),
            typesafe_default_model: default_typesafe_default_model(),
        }
    }
}

/// Default SystemOne base URL when `typesafe_base_url` is omitted.
pub const DEFAULT_TYPESAFE_BASE_URL: &str = "https://api.typesafe.ai";

/// Default SystemOne model when `typesafe_default_model` is omitted.
pub const DEFAULT_TYPESAFE_MODEL: &str = "jev-latest";

/// Build the shared SystemOne (Jev) handle from config; API key from
/// `TYPESAFE_API_KEY`.
pub fn shared_systemone(cfg: &SystemOneConfig) -> Result<patterns::systemone::SharedSystemOne> {
    let api_key = std::env::var("TYPESAFE_API_KEY").context("TYPESAFE_API_KEY is not set")?;
    Ok(patterns::systemone::SharedSystemOne::new(
        cfg.typesafe_base_url.clone(),
        api_key,
        cfg.typesafe_default_model.clone(),
        patterns::llm_cli::ConcurrencyLimits::default(),
    ))
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
            personal_info: PersonalInfo {
                location: crate::region::Region::Europe,
                cv: "./cv.md".to_string(),
            },
            pause_ms: 2000,
            browser: BrowserConfig {
                bin: SAMPLE_BROWSER_BIN.to_string(),
            },
            llm: LlmConfig {
                bin: SAMPLE_LLM_BIN.to_string(),
                args: SAMPLE_LLM_ARGS.to_string(),
            },
            providers: Providers {
                upwork: ProviderConfig {
                    urls: vec!["https://www.upwork.com/nx/search/jobs/?q=trading&sort=recency&per_page=50&t=0&hourly_rate=60-".to_string()],
                },
                nofluffjobs: ProviderConfig {
                    urls: vec!["https://nofluffjobs.com/remote?criteria=employment%3Db2b%20salary%3Eeur8000m%20jobLanguage%3Den&sort=newest".to_string()],
                },
                efinancialcareers: ProviderConfig {
                    urls: vec!["https://www.efinancialcareers.com/jobs/remote/python?pageSize=50&filters.postedDate=SEVEN&language=en".to_string()],
                },
                linkedin: ProviderConfig {
                    urls: vec!["https://www.linkedin.com/jobs/search/?f_I=4&f_T=9%2C25201%2C39&f_TPR=r200000&f_WT=2&geoId=92000000".to_string()],
                },
                workatastartup: ProviderConfig {
                    urls: vec!["https://www.workatastartup.com/companies?role=eng&remote=only&usVisaNotRequired=true&sortBy=created_desc".to_string()],
                },
                wellfound: ProviderConfig {
                    urls: vec!["https://wellfound.com/role/l/software-engineer/europe".to_string()],
                },
            },
            systemone: SystemOneConfig::default(),
        }
    }
}
