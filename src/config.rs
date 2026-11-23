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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// OpenAI-compatible base URL, e.g. `https://api.deepseek.com`.
    pub base_url: String,
    /// Model id at that provider, e.g. `deepseek-flash`.
    pub model: String,
    /// Name of the environment variable holding the API key (never the key).
    pub api_key_env: String,
}

/// Build the shared chat handle from config; one per process (shared concurrency
/// cap across scrapers).
///
/// # Errors
/// If `api_key_env` is unset or blank: a missing key fails at startup, not on the
/// first paid extraction.
pub fn shared_llm(cfg: &LlmConfig) -> Result<patterns::llm::ChatLlm> {
    let api_key = std::env::var(&cfg.api_key_env)
        .with_context(|| format!("environment variable {} is not set", cfg.api_key_env))?;
    anyhow::ensure!(
        !api_key.trim().is_empty(),
        "environment variable {} is empty",
        cfg.api_key_env
    );
    Ok(patterns::llm::ChatLlm::new(
        cfg.base_url.clone(),
        api_key,
        cfg.model.clone(),
        patterns::limits::ConcurrencyLimits::default(),
    ))
}

/// Placeholder written into the sample config by `jobsearch init`.
pub const SAMPLE_BROWSER_BIN: &str = "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser";

/// Sample base URL written into the sample config by `jobsearch init`.
pub const SAMPLE_LLM_BASE_URL: &str = "https://api.deepseek.com";

/// Sample model id written into the sample config by `jobsearch init`.
pub const SAMPLE_LLM_MODEL: &str = "deepseek-flash";

/// Sample API-key env var name written into the sample config by `jobsearch init`.
pub const SAMPLE_LLM_API_KEY_ENV: &str = "DEEPSEEK_API_KEY";

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
            },
            pause_ms: 2000,
            browser: BrowserConfig {
                bin: SAMPLE_BROWSER_BIN.to_string(),
            },
            llm: LlmConfig {
                base_url: SAMPLE_LLM_BASE_URL.to_string(),
                model: SAMPLE_LLM_MODEL.to_string(),
                api_key_env: SAMPLE_LLM_API_KEY_ENV.to_string(),
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
        }
    }
}
