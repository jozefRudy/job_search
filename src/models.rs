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
    pub description: String,
    #[serde(default)]
    pub requirements: String,
    #[serde(default)]
    pub nice_to_have: String,
    #[serde(default)]
    pub offer_valid_until: String,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub posted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ValueEnum)]
#[clap(rename_all = "lower")]
#[sqlx(rename_all = "lowercase")]
pub enum Platform {
    NoFluffJobs,
    Upwork,
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
    pub budget: Option<String>,
    pub tags: Vec<String>,
    pub raw: Data,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub note: Option<String>,
    pub applied_at: Option<DateTime<Utc>>,
}

/// Parsed budget range with consistent formatting.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Budget {
    pub min: u32,
    pub max: u32,
    pub currency: String,
    pub period: Option<String>,
}

impl fmt::Display for Budget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - {} {}", self.min, self.max, self.currency)?;
        if let Some(p) = &self.period {
            write!(f, "/{}", p)?;
        }
        Ok(())
    }
}

impl Budget {
    /// Parse budget strings like "7 069 – 9 426 EUR" or "$50-$100/hr".
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.replace(['\u{00a0}', '\u{2007}', '\u{202f}'], " ");
        let trimmed = s.trim();

        let (base, period) = if let Some((b, p)) = trimmed.rsplit_once('/') {
            (b.trim(), Some(p.trim().to_string()))
        } else {
            (trimmed, None)
        };

        for (prefix, cur) in [('$', "USD"), ('€', "EUR")] {
            if let Some(rest) = base.strip_prefix(prefix) {
                return Self::parse_range(rest.trim(), cur, period);
            }
        }
        for cur in ["EUR", "PLN", "USD", "GBP", "CHF"] {
            if let Some(prefix) = base.strip_suffix(cur) {
                return Self::parse_range(prefix.trim(), cur, period);
            }
        }

        None
    }

    fn parse_range(s: &str, currency: &str, period: Option<String>) -> Option<Self> {
        for sep in ['–', '-'] {
            if let Some((a, b)) = s.split_once(sep) {
                let min = Self::extract_number(a)?;
                let max = Self::extract_number(b)?;
                return Some(Budget {
                    min,
                    max,
                    currency: currency.to_string(),
                    period,
                });
            }
        }
        let val = Self::extract_number(s)?;
        Some(Budget {
            min: val,
            max: val,
            currency: currency.to_string(),
            period,
        })
    }

    fn extract_number(s: &str) -> Option<u32> {
        s.chars()
            .filter(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_parse_nofluff() {
        let b = Budget::parse("7 069 – 9 426 EUR").unwrap();
        assert_eq!(b.min, 7069);
        assert_eq!(b.max, 9426);
        assert_eq!(b.currency, "EUR");
        assert_eq!(b.period, None);
        assert_eq!(b.to_string(), "7069 - 9426 EUR");
    }

    #[test]
    fn test_budget_parse_nofluff_nbsp() {
        let b = Budget::parse("7\u{00a0}069 – 9\u{00a0}426 EUR").unwrap();
        assert_eq!(b.min, 7069);
        assert_eq!(b.max, 9426);
        assert_eq!(b.currency, "EUR");
    }

    #[test]
    fn test_budget_parse_upwork_hourly() {
        let b = Budget::parse("$50-$100/hr").unwrap();
        assert_eq!(b.min, 50);
        assert_eq!(b.max, 100);
        assert_eq!(b.currency, "USD");
        assert_eq!(b.period, Some("hr".to_string()));
        assert_eq!(b.to_string(), "50 - 100 USD/hr");
    }

    #[test]
    fn test_budget_parse_upwork_fixed() {
        let b = Budget::parse("$5,000").unwrap();
        assert_eq!(b.min, 5000);
        assert_eq!(b.max, 5000);
        assert_eq!(b.currency, "USD");
    }

    #[test]
    fn test_budget_parse_pln() {
        let b = Budget::parse("15 000 – 20 000 PLN").unwrap();
        assert_eq!(b.min, 15000);
        assert_eq!(b.max, 20000);
        assert_eq!(b.currency, "PLN");
    }

    #[test]
    fn test_budget_parse_euro_prefix() {
        let b = Budget::parse("€50-€100").unwrap();
        assert_eq!(b.min, 50);
        assert_eq!(b.max, 100);
        assert_eq!(b.currency, "EUR");
        assert_eq!(b.to_string(), "50 - 100 EUR");
    }

    #[test]
    fn test_budget_parse_unknown_returns_none() {
        assert!(Budget::parse("Negotiable").is_none());
        assert!(Budget::parse("").is_none());
    }
}
