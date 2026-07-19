use crate::language::LanguageService;
use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::ValueEnum;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::LazyLock;
use utoipa::IntoParams;
use utoipa::ToSchema;

/// Platform-specific scraped data stored on each job.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "platform", rename_all = "lowercase")]
pub enum Data {
    Upwork { detail: UpworkJobDetail },
    Nofluffjobs { detail: NoFluffJobDetail },
    Efinancialcareers { detail: EfinancialcareersJobDetail },
    Hackernews { detail: HackerNewsJobDetail },
    LinkedIn { detail: LinkedInJobDetail },
}

/// Full detail scraped from an individual Hacker News "Who is hiring?" comment.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct HackerNewsJobDetail {
    pub author: String,
    pub author_threads_url: String,
    pub company: Option<String>,
    pub role: Option<String>,
    pub location: Option<String>,
}

/// Full detail scraped from an individual eFinancialCareers job page.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EfinancialcareersJobDetail {
    pub company: String,
    pub location: String,
    pub employment_type: String,
    pub work_arrangement_type: String,
    pub salary: String,
    pub description: String,
    pub posted_at: DateTime<Utc>,
}

impl Default for EfinancialcareersJobDetail {
    fn default() -> Self {
        Self {
            company: String::new(),
            location: String::new(),
            employment_type: String::new(),
            work_arrangement_type: String::new(),
            salary: String::new(),
            description: String::new(),
            posted_at: Utc::now(),
        }
    }
}

/// Full detail scraped from a LinkedIn job posting via Voyager API.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LinkedInJobDetail {
    pub company: String,
    pub location: String,
    pub workplace_type: String,
    pub employment_type: String,
    pub job_function: String,
    pub industries: String,
    pub description: String,
    pub salary: String,
    pub posted_at: DateTime<Utc>,
}

impl Default for LinkedInJobDetail {
    fn default() -> Self {
        Self {
            company: String::new(),
            location: String::new(),
            workplace_type: String::new(),
            employment_type: String::new(),
            job_function: String::new(),
            industries: String::new(),
            description: String::new(),
            salary: String::new(),
            posted_at: Utc::now(),
        }
    }
}

/// Full detail scraped from an individual Upwork job page.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpworkJobDetail {
    pub proposals: String,
    pub last_viewed: Option<DateTime<Utc>>,
    pub interviewing: String,
    pub invites_sent: String,
    pub unanswered_invites: String,
    pub description: String,
    pub exact_budget: String,
    pub experience_level: String,
    pub hires: String,
    pub project_type: String,
    pub duration: String,
    pub hours_per_week: String,
    pub tags: Vec<String>,
    pub posted_at: DateTime<Utc>,
}

impl Default for UpworkJobDetail {
    fn default() -> Self {
        Self {
            proposals: String::new(),
            last_viewed: None,
            interviewing: String::new(),
            invites_sent: String::new(),
            unanswered_invites: String::new(),
            description: String::new(),
            exact_budget: String::new(),
            experience_level: String::new(),
            hires: String::new(),
            project_type: String::new(),
            duration: String::new(),
            hours_per_week: String::new(),
            tags: Vec::new(),
            posted_at: Utc::now(),
        }
    }
}

static RELATIVE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(yesterday|last\s+(?:week|month|quarter))|(\d+)\s*([a-z]+)(?:\s+ago)?")
        .unwrap()
});

pub fn parse_relative_time(text: &str) -> Option<DateTime<Utc>> {
    let now = Utc::now();
    let caps = RELATIVE_RE.captures(text)?;

    if let Some(special) = caps.get(1) {
        match special.as_str().to_lowercase().as_str() {
            "yesterday" => return Some(now - chrono::Duration::days(1)),
            "last week" => return Some(now - chrono::Duration::days(7)),
            "last month" => return Some(now - chrono::Duration::days(30)),
            "last quarter" => return Some(now - chrono::Duration::days(90)),
            _ => {}
        }
    }

    let n: i64 = caps.get(2)?.as_str().parse().ok()?;
    let unit = caps.get(3)?.as_str().to_lowercase();
    unit_to_duration(&unit, n, now)
}

fn unit_to_duration(unit: &str, n: i64, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    match unit {
        "m" | "min" | "mins" | "minute" | "minutes" => Some(now - chrono::Duration::minutes(n)),
        "h" | "hr" | "hrs" | "hour" | "hours" => Some(now - chrono::Duration::hours(n)),
        "d" | "day" | "days" => Some(now - chrono::Duration::days(n)),
        "w" | "week" | "weeks" => Some(now - chrono::Duration::days(n * 7)),
        "mo" | "month" | "months" => Some(now - chrono::Duration::days(n * 30)),
        "quarter" | "quarters" => Some(now - chrono::Duration::days(n * 90)),
        _ => None,
    }
}

/// Full detail scraped from an individual `NoFluffJobs` job page.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NoFluffJobDetail {
    pub company: String,
    pub seniority: String,
    pub locations: Vec<String>,
    pub must_have: Vec<String>,
    pub description: String,
    pub requirements: String,
    pub nice_to_have: String,
    pub offer_valid_until: String,
    pub languages: Vec<String>,
    pub posted_at: DateTime<Utc>,
    pub employment_type: Option<String>,
}

impl Default for NoFluffJobDetail {
    fn default() -> Self {
        Self {
            company: String::new(),
            seniority: String::new(),
            locations: Vec::new(),
            must_have: Vec::new(),
            description: String::new(),
            requirements: String::new(),
            nice_to_have: String::new(),
            offer_valid_until: String::new(),
            languages: Vec::new(),
            posted_at: Utc::now(),
            employment_type: None,
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ValueEnum, ToSchema,
)]
#[clap(rename_all = "lower")]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Efinancialcareers,
    Hackernews,
    NoFluffJobs,
    Upwork,
    LinkedIn,
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Platform::Efinancialcareers => write!(f, "efinancialcareers"),
            Platform::Hackernews => write!(f, "hackernews"),
            Platform::NoFluffJobs => write!(f, "nofluffjobs"),
            Platform::Upwork => write!(f, "upwork"),
            Platform::LinkedIn => write!(f, "linkedin"),
        }
    }
}

impl From<String> for Platform {
    fn from(s: String) -> Self {
        match s.as_str() {
            "efinancialcareers" => Platform::Efinancialcareers,
            "hackernews" => Platform::Hackernews,
            "nofluffjobs" => Platform::NoFluffJobs,
            "upwork" => Platform::Upwork,
            "linkedin" => Platform::LinkedIn,
            _ => panic!("unknown platform in db: '{s}'"),
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum, ToSchema, sqlx::Type,
)]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Rating {
    Liked,
    Disliked,
    Neutral,
}

impl From<String> for Rating {
    fn from(s: String) -> Self {
        match s.as_str() {
            "liked" => Rating::Liked,
            "disliked" => Rating::Disliked,
            "neutral" => Rating::Neutral,
            _ => panic!("unknown rating in db: '{s}'"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub total: i64,
}

/**
 * Sorts available in the API and CLI.
 *
 * `Relevance` is used only by the vector search path and is not DB-sortable.
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Sort {
    #[default]
    Created,
    UpworkViewed,
    Applied,
    Relevance,
}

impl Sort {
    #[must_use]
    pub fn order_by_sql(&self) -> &'static str {
        match self {
            Sort::Created => "j.created_at DESC",
            Sort::UpworkViewed => "j.upwork_last_viewed_at DESC NULLS LAST",
            Sort::Applied => "r.applied_at DESC NULLS LAST",
            // `Relevance` is handled by the vector search path, not SQL.
            Sort::Relevance => unreachable!("Relevance sort is not SQL-sortable"),
        }
    }
}

fn default_page() -> usize {
    1
}
fn default_page_size() -> usize {
    20
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListQuery {
    pub platform: Option<Platform>,
    pub rating: Option<Rating>,
    pub applied: Option<bool>,
    pub remote: Option<bool>,
    pub search: Option<String>,
    #[serde(default)]
    pub sort_by: Sort,
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Job {
    pub id: i64,
    pub platform: Platform,
    pub external_id: String,
    pub title: String,
    pub description: Option<String>,
    pub url: String,
    pub budget: Option<String>,
    pub tags: Vec<String>,
    pub raw: Data,
    pub company: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub note: Option<String>,
    pub rating: Rating,
    pub applied_at: Option<DateTime<Utc>>,
    pub remote: bool,
}

impl Job {
    /// Single text blob from all available advert text for language detection and embedding.
    #[must_use]
    pub fn advert_text(&self) -> String {
        let mut text = self.title.clone();
        if let Some(d) = &self.description {
            text.push(' ');
            text.push_str(d);
        }
        match &self.raw {
            Data::Upwork { detail } => {
                text.push(' ');
                text.push_str(&detail.description);
            }
            Data::Nofluffjobs { detail } => {
                text.push(' ');
                text.push_str(&detail.description);
                text.push(' ');
                text.push_str(&detail.requirements);
            }
            Data::Efinancialcareers { detail } => {
                text.push(' ');
                text.push_str(&detail.description);
            }
            Data::Hackernews { .. } => {}
            Data::LinkedIn { detail } => {
                text.push(' ');
                text.push_str(&detail.description);
            }
        }
        text
    }
}

/// Detect whether a job advert is English.
pub async fn classify_language(svc: &LanguageService, job: &Job) -> Result<bool> {
    svc.detect(&job.advert_text()).await
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct JobListResponse {
    pub jobs: Vec<Job>,
    pub total: usize,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RateRequest {
    pub rating: Rating,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ApplyRequest {
    pub applied: bool,
}

/// Parsed recency like "1d" or "4w". Stores days.
#[derive(Debug, Clone)]
pub struct Recency(pub i64);

impl std::str::FromStr for Recency {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.len() < 2 {
            anyhow::bail!("recency must be like 1d or 4w, got '{s}'");
        }
        let (num, unit) = s.split_at(s.len() - 1);
        let n: i64 = num
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid recency number '{num}'"))?;
        let days = match unit {
            "d" => n,
            "w" => n * 7,
            _ => anyhow::bail!("recency unit must be 'd' or 'w', got '{unit}'"),
        };
        Ok(Recency(days))
    }
}

/// Filter criteria for job lists.
#[derive(Debug, Clone, Default)]
pub struct JobFilter {
    pub platform: Option<Platform>,
    pub applied: Option<bool>,
    pub rating: Option<Rating>,
    pub remote: Option<bool>,
}

/// Parsed budget value with consistent formatting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Budget {
    Range {
        min: u32,
        max: u32,
        currency: String,
        period: Option<String>,
    },
    Single {
        amount: u32,
        currency: String,
        period: Option<String>,
    },
}

impl fmt::Display for Budget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Budget::Range {
                min,
                max,
                currency,
                period,
            } => {
                write!(f, "{min} - {max} {currency}")?;
                if let Some(p) = period {
                    write!(f, "/{p}")?;
                }
            }
            Budget::Single {
                amount,
                currency,
                period,
            } => {
                write!(f, "{amount} {currency}")?;
                if let Some(p) = period {
                    write!(f, "/{p}")?;
                }
            }
        }
        Ok(())
    }
}

impl Budget {
    /// Parse budget strings like "7 069 – 9 426 EUR" or "$50-$100/hr".
    /// `default_period` is applied when the string does not already specify one.
    #[must_use]
    pub fn parse(s: &str, default_period: Option<&str>) -> Option<Self> {
        let s = Self::normalize(s);
        Self::parse_code_prefix_range(&s, default_period)
            .or_else(|| Self::parse_symbol_prefix(&s, default_period))
            .or_else(|| Self::parse_suffix_currency(&s, default_period))
    }

    fn resolve_period(explicit: Option<String>, default: Option<&str>) -> Option<String> {
        explicit.or_else(|| default.map(std::string::ToString::to_string))
    }

    /// Parse currency-code-prefixed ranges like "USD120000 - USD140000 per annum".
    fn parse_code_prefix_range(s: &str, default_period: Option<&str>) -> Option<Budget> {
        static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
            regex::Regex::new(
                r"(?i)\b(USD|EUR|GBP|PLN|CHF)\s*(\d[\d,]*[kKmM]?)\s*-\s*(?:USD|EUR|GBP|PLN|CHF)\s*(\d[\d,]*[kKmM]?)\b",
            )
            .unwrap()
        });

        let (base, period) = Self::split_period(s);
        let caps = RE.captures(base.trim())?;
        let min = Self::parse_number(&caps[2])?;
        let max = Self::parse_number(&caps[3])?;
        Some(if min == max {
            Budget::Single {
                amount: min,
                currency: caps[1].to_ascii_uppercase(),
                period: Self::resolve_period(period, default_period),
            }
        } else {
            Budget::Range {
                min,
                max,
                currency: caps[1].to_ascii_uppercase(),
                period: Self::resolve_period(period, default_period),
            }
        })
    }

    fn normalize(s: &str) -> String {
        s.replace(['\u{00a0}', '\u{2007}', '\u{202f}'], " ")
    }

    fn split_period(s: &str) -> (&str, Option<String>) {
        let (base, period) = s.rsplit_once('/').unwrap_or((s, ""));
        let period = period.trim();
        if period.is_empty() {
            (base.trim(), None)
        } else {
            (base.trim(), Some(period.to_string()))
        }
    }

    fn parse_number(s: &str) -> Option<u32> {
        static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
            regex::Regex::new(r"(\d[\d\s,]*)(?:\.(\d+))?\s*([kKmM])?\b").unwrap()
        });

        let s = Self::normalize(s);
        let caps = RE.captures(s.trim())?;

        let integer: u32 = caps[1]
            .chars()
            .filter(char::is_ascii_digit)
            .collect::<String>()
            .parse()
            .ok()?;

        let fraction_part = caps.get(2).map(|m| m.as_str());
        let fraction_digits = fraction_part.map_or(0, str::len);
        let fraction: u32 = fraction_part.map_or(0, |s| {
            s.chars()
                .filter(char::is_ascii_digit)
                .collect::<String>()
                .parse()
                .unwrap_or(0)
        });

        let fraction_digits_u32 = u32::try_from(fraction_digits).unwrap_or(u32::MAX);

        let multiplier: u64 = match caps
            .get(3)
            .map(|m| m.as_str().to_ascii_lowercase())
            .as_deref()
        {
            Some("k") => 1_000,
            Some("m") => 1_000_000,
            _ => 1,
        };

        if fraction_digits == 0 {
            (u64::from(integer) * multiplier).try_into().ok()
        } else {
            let scale = 10_u64.pow(fraction_digits_u32);
            let value = (u64::from(integer) * scale + u64::from(fraction)) * multiplier / scale;
            value.try_into().ok()
        }
    }

    fn parse_range(s: &str) -> Option<(u32, u32)> {
        static RANGE_RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
            regex::Regex::new(
                r"(?:\$|€|£|\b(?:USD|EUR|GBP|PLN|CHF)\b\s*)?(\d[\d\s,.$£€]*[kKmM]?)\s*(?:–|-|\s+to\s+)\s*(?:\$|€|£|\b(?:USD|EUR|GBP|PLN|CHF)\b\s*)?(\d[\d\s,.$£€]*[kKmM]?)\b",
            )
            .unwrap()
        });

        static CURRENCY_CODE_RANGE_RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(
            || {
                regex::Regex::new(
                    r"(?i)(USD|EUR|GBP|PLN|CHF)\s*(\d[\d,]*[kKmM]?)\s*-\s*(USD|EUR|GBP|PLN|CHF)\s*(\d[\d,]*[kKmM]?)",
                )
                .unwrap()
            },
        );

        if let Some(caps) = CURRENCY_CODE_RANGE_RE.captures(s) {
            let min = Self::parse_number(&caps[2])?;
            let max = Self::parse_number(&caps[4])?;
            return Some((min, max));
        }

        if let Some(caps) = RANGE_RE.captures(s) {
            let min = Self::parse_number(&caps[1])?;
            let max = Self::parse_number(&caps[2])?;
            return Some((min, max));
        }

        let val = Self::parse_number(s)?;
        Some((val, val))
    }

    fn parse_symbol_prefix(s: &str, default_period: Option<&str>) -> Option<Budget> {
        let (base, period) = Self::split_period(s);

        let (currency, _) = if let Some(rest) = base.strip_prefix('$') {
            ("USD", rest)
        } else {
            let rest = base.strip_prefix('€')?;
            ("EUR", rest)
        };

        let (min, max) = Self::parse_range(base)?;
        Some(if min == max {
            Budget::Single {
                amount: min,
                currency: currency.to_string(),
                period: Self::resolve_period(period, default_period),
            }
        } else {
            Budget::Range {
                min,
                max,
                currency: currency.to_string(),
                period: Self::resolve_period(period, default_period),
            }
        })
    }

    fn parse_suffix_currency(s: &str, default_period: Option<&str>) -> Option<Budget> {
        let (base, period) = Self::split_period(s);

        for currency in ["EUR", "PLN", "USD", "GBP", "CHF"] {
            if base.contains(currency) {
                let prefix = base
                    .trim()
                    .strip_suffix(currency)
                    .or_else(|| base.trim().strip_prefix(currency))
                    .unwrap_or(base.trim());
                let (min, max) = Self::parse_range(prefix.trim())?;
                return Some(if min == max {
                    Budget::Single {
                        amount: min,
                        currency: currency.to_string(),
                        period: Self::resolve_period(period, default_period),
                    }
                } else {
                    Budget::Range {
                        min,
                        max,
                        currency: currency.to_string(),
                        period: Self::resolve_period(period, default_period),
                    }
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_parse_nofluff() {
        let b = Budget::parse("7 069 – 9 426 EUR", Some("mo")).unwrap();
        assert_eq!(b.to_string(), "7069 - 9426 EUR/mo");
    }

    #[test]
    fn test_budget_parse_nofluff_nbsp() {
        let b = Budget::parse("7\u{00a0}069 – 9\u{00a0}426 EUR", Some("mo")).unwrap();
        assert_eq!(b.to_string(), "7069 - 9426 EUR/mo");
    }

    #[test]
    fn test_budget_parse_upwork_hourly() {
        let b = Budget::parse("$50-$100/hr", Some("hr")).unwrap();
        assert_eq!(b.to_string(), "50 - 100 USD/hr");
    }

    #[test]
    fn test_budget_parse_upwork_fixed() {
        let b = Budget::parse("$5,000", None).unwrap();
        assert_eq!(b.to_string(), "5000 USD");
    }

    #[test]
    fn test_budget_parse_pln() {
        let b = Budget::parse("15 000 – 20 000 PLN", Some("mo")).unwrap();
        assert_eq!(b.to_string(), "15000 - 20000 PLN/mo");
    }

    #[test]
    fn test_budget_parse_euro_prefix() {
        let b = Budget::parse("€50-€100", Some("mo")).unwrap();
        assert_eq!(b.to_string(), "50 - 100 EUR/mo");
    }

    #[test]
    fn test_budget_parse_to_range() {
        let b = Budget::parse("$130,530 to 221,920 USD", Some("hr")).unwrap();
        assert_eq!(b.to_string(), "130530 - 221920 USD/hr");
    }

    #[test]
    fn test_budget_parse_nofluff_space_separated_numbers() {
        let b = Budget::parse("15 000 – 18 000 PLN", Some("mo")).unwrap();
        assert_eq!(b.to_string(), "15000 - 18000 PLN/mo");
    }

    #[test]
    fn test_budget_parse_upwork_hourly_with_period() {
        let b = Budget::parse("$125 - $200/hr", Some("hr")).unwrap();
        assert_eq!(b.to_string(), "125 - 200 USD/hr");
    }

    #[test]
    fn test_budget_parse_unknown_returns_none() {
        assert!(Budget::parse("Negotiable", None).is_none());
        assert!(Budget::parse("", None).is_none());
    }

    #[test]
    fn test_budget_parse_k_suffix() {
        let b = Budget::parse("$100k", Some("year")).unwrap();
        assert_eq!(b.to_string(), "100000 USD/year");
    }

    #[test]
    fn test_budget_parse_k_range() {
        let b = Budget::parse("$120k - $150k", Some("year")).unwrap();
        assert_eq!(b.to_string(), "120000 - 150000 USD/year");
    }

    #[test]
    fn test_budget_parse_k_uppercase() {
        let b = Budget::parse("€80K", Some("year")).unwrap();
        assert_eq!(b.to_string(), "80000 EUR/year");
    }

    #[test]
    fn test_budget_parse_efinancialcareers_usd_per_annum() {
        let b = Budget::parse("USD120000 - USD140000 per annum", Some("year")).unwrap();
        assert_eq!(b.to_string(), "120000 - 140000 USD/year");
    }

    #[test]
    fn test_budget_parse_efinancialcareers_gbp_per_annum() {
        let b = Budget::parse("GBP90000 - GBP110000 per annum", Some("year")).unwrap();
        assert_eq!(b.to_string(), "90000 - 110000 GBP/year");
    }

    #[test]
    fn test_parse_relative_time() {
        let now = Utc::now();

        let cases = [
            ("20m ago", chrono::Duration::minutes(20)),
            ("20 minutes ago", chrono::Duration::minutes(20)),
            ("1h ago", chrono::Duration::hours(1)),
            ("3 hours ago", chrono::Duration::hours(3)),
            ("2d ago", chrono::Duration::days(2)),
            ("1 day ago", chrono::Duration::days(1)),
            ("3w ago", chrono::Duration::days(21)),
            ("1 week ago", chrono::Duration::days(7)),
            ("yesterday", chrono::Duration::days(1)),
            ("last quarter", chrono::Duration::days(90)),
            ("1 quarter ago", chrono::Duration::days(90)),
        ];

        for (input, expected) in cases {
            let result = parse_relative_time(input);
            assert!(
                result.is_some(),
                "expected parse_relative_time({input:?}) to succeed"
            );
            let diff = now - result.unwrap();
            assert!(
                (diff - expected).num_seconds().abs() < 2,
                "expected ~{expected:?} for {input:?}, got {diff:?}"
            );
        }

        assert!(parse_relative_time("").is_none());
        assert!(parse_relative_time("unknown").is_none());
    }

    #[test]
    fn test_parse_relative_time_special_and_long() {
        let now = Utc::now();

        let cases = [
            ("5 minutes ago", chrono::Duration::minutes(5)),
            ("1 hour ago", chrono::Duration::hours(1)),
            ("3 hours ago", chrono::Duration::hours(3)),
            ("yesterday", chrono::Duration::days(1)),
            ("2 days ago", chrono::Duration::days(2)),
            ("last week", chrono::Duration::days(7)),
            ("3 weeks ago", chrono::Duration::days(21)),
            ("last month", chrono::Duration::days(30)),
            ("2 months ago", chrono::Duration::days(60)),
            ("last quarter", chrono::Duration::days(90)),
            ("1 quarter ago", chrono::Duration::days(90)),
        ];

        for (input, expected_duration) in cases {
            let result = parse_relative_time(input);
            assert!(
                result.is_some(),
                "expected parse_relative_time({input:?}) to succeed"
            );
            let diff = now - result.unwrap();
            assert!(
                (diff - expected_duration).num_seconds().abs() < 2,
                "expected ~{expected_duration:?} for {input:?}, got {diff:?}"
            );
        }
    }

    #[test]
    fn test_parse_relative_time_with_prefixes() {
        let now = Utc::now();

        let cases = [
            ("Posted 5m ago", chrono::Duration::minutes(5)),
            ("Posted 1 hour ago", chrono::Duration::hours(1)),
            ("viewed 2h ago", chrono::Duration::hours(2)),
            ("last viewed by client: 3d ago", chrono::Duration::days(3)),
            ("Last Viewed By Client 1w ago", chrono::Duration::days(7)),
        ];

        for (input, expected) in cases {
            let result = parse_relative_time(input);
            assert!(
                result.is_some(),
                "expected parse_relative_time({input:?}) to succeed"
            );
            let diff = now - result.unwrap();
            assert!(
                (diff - expected).num_seconds().abs() < 2,
                "expected ~{expected:?} for {input:?}, got {diff:?}"
            );
        }
    }

    #[tokio::test]
    async fn test_classify_language_detects_english() {
        let svc = LanguageService::new();
        let raw = Data::Upwork {
            detail: UpworkJobDetail {
                description: "We are looking for a Rust developer to build web services."
                    .to_string(),
                ..Default::default()
            },
        };
        let job = Job {
            id: 0,
            platform: Platform::Upwork,
            external_id: "ext".to_string(),
            title: "Rust Developer".to_string(),
            description: Some("Remote full-time role".to_string()),
            url: "https://example.com".to_string(),
            budget: None,
            tags: Vec::new(),
            raw,
            company: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            note: None,
            rating: Rating::Neutral,
            applied_at: None,
            remote: true,
        };
        let en = classify_language(&svc, &job)
            .await
            .expect("language classification should succeed");
        assert!(en);
    }
}
