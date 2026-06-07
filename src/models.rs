use chrono::{DateTime, Utc};
use clap::ValueEnum;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::LazyLock;

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
    #[serde(default, deserialize_with = "deserialize_relative_time")]
    pub last_viewed: Option<DateTime<Utc>>,
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

pub fn deserialize_relative_time<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = Option::<String>::deserialize(deserializer)?;
    Ok(s.and_then(|s| parse_relative_time(&s)))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
pub enum Rating {
    Liked,
    Disliked,
    Neutral,
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
    pub liked: Option<bool>,
    pub applied_at: Option<DateTime<Utc>>,
}

/// Parsed recency like "1d" or "4w". Stores days.
#[derive(Debug, Clone)]
pub struct Recency(pub i64);

impl std::str::FromStr for Recency {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.len() < 2 {
            anyhow::bail!("recency must be like 1d or 4w, got '{}'", s);
        }
        let (num, unit) = s.split_at(s.len() - 1);
        let n: i64 = num
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid recency number '{}'", num))?;
        let days = match unit {
            "d" => n,
            "w" => n * 7,
            _ => anyhow::bail!("recency unit must be 'd' or 'w', got '{}'", unit),
        };
        Ok(Recency(days))
    }
}

/// Filter criteria for job lists.
#[derive(Debug, Clone, Default)]
pub struct JobFilter {
    pub recency: Option<Recency>,
    pub applied: Option<bool>,
    pub liked: Option<Rating>,
}

impl JobFilter {
    pub fn apply(&self, jobs: Vec<Job>) -> Vec<Job> {
        let mut jobs = jobs;

        if let Some(Recency(days)) = &self.recency {
            let cutoff = Utc::now() - chrono::Duration::days(*days);
            jobs.retain(|j| j.created_at >= cutoff);
        }

        match self.applied {
            Some(true) => jobs.retain(|j| j.applied_at.is_some()),
            Some(false) => jobs.retain(|j| j.applied_at.is_none()),
            None => {}
        }

        match self.liked {
            Some(Rating::Liked) => jobs.retain(|j| j.liked == Some(true)),
            Some(Rating::Disliked) => jobs.retain(|j| j.liked == Some(false)),
            Some(Rating::Neutral) => jobs.retain(|j| j.liked.is_none()),
            None => {}
        }

        jobs
    }
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
                "expected parse_relative_time({:?}) to succeed",
                input
            );
            let diff = now - result.unwrap();
            assert!(
                (diff - expected).num_seconds().abs() < 2,
                "expected ~{:?} for {:?}, got {:?}",
                expected,
                input,
                diff
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
                "expected parse_relative_time({:?}) to succeed",
                input
            );
            let diff = now - result.unwrap();
            assert!(
                (diff - expected_duration).num_seconds().abs() < 2,
                "expected ~{:?} for {:?}, got {:?}",
                expected_duration,
                input,
                diff
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
                "expected parse_relative_time({:?}) to succeed",
                input
            );
            let diff = now - result.unwrap();
            assert!(
                (diff - expected).num_seconds().abs() < 2,
                "expected ~{:?} for {:?}, got {:?}",
                expected,
                input,
                diff
            );
        }
    }

    #[test]
    fn test_job_filter_rating() {
        fn job(id: i64, liked: Option<bool>) -> Job {
            Job {
                id: Some(id),
                platform: Platform::Upwork,
                external_id: format!("j{id}"),
                title: format!("Job {id}"),
                description: None,
                url: "https://e.com".into(),
                budget: None,
                tags: vec![],
                raw: Data::Upwork {
                    detail: UpworkJobDetail::default(),
                },
                created_at: Utc::now(),
                updated_at: Utc::now(),
                liked,
                note: None,
                applied_at: None,
            }
        }

        let jobs = vec![job(1, Some(true)), job(2, Some(false)), job(3, None)];

        let ids = |f: JobFilter| {
            f.apply(jobs.clone())
                .into_iter()
                .map(|j| j.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(
            ids(JobFilter {
                liked: Some(Rating::Liked),
                ..Default::default()
            }),
            vec![Some(1)]
        );
        assert_eq!(
            ids(JobFilter {
                liked: Some(Rating::Disliked),
                ..Default::default()
            }),
            vec![Some(2)]
        );
        assert_eq!(
            ids(JobFilter {
                liked: Some(Rating::Neutral),
                ..Default::default()
            }),
            vec![Some(3)]
        );
        assert_eq!(
            ids(JobFilter {
                liked: None,
                ..Default::default()
            }),
            vec![Some(1), Some(2), Some(3)]
        );
    }

    #[test]
    fn test_job_filter_recency() {
        fn job(id: i64, created_at: DateTime<Utc>) -> Job {
            Job {
                id: Some(id),
                platform: Platform::Upwork,
                external_id: format!("j{id}"),
                title: format!("Job {id}"),
                description: None,
                url: "https://e.com".into(),
                budget: None,
                tags: vec![],
                raw: Data::Upwork {
                    detail: UpworkJobDetail::default(),
                },
                created_at,
                updated_at: Utc::now(),
                liked: None,
                note: None,
                applied_at: None,
            }
        }

        let jobs = vec![
            job(1, Utc::now() - chrono::Duration::hours(1)),
            job(2, Utc::now() - chrono::Duration::days(2)),
            job(3, Utc::now() - chrono::Duration::days(10)),
        ];

        let ids = |f: JobFilter| {
            f.apply(jobs.clone())
                .into_iter()
                .map(|j| j.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(
            ids(JobFilter {
                recency: Some(Recency(1)),
                ..Default::default()
            }),
            vec![Some(1)]
        );
        assert_eq!(
            ids(JobFilter {
                recency: Some(Recency(5)),
                ..Default::default()
            }),
            vec![Some(1), Some(2)]
        );
        assert_eq!(
            ids(JobFilter {
                recency: None,
                ..Default::default()
            }),
            vec![Some(1), Some(2), Some(3)]
        );
    }

    #[test]
    fn test_job_filter_applied() {
        fn job(id: i64, applied_at: Option<DateTime<Utc>>) -> Job {
            Job {
                id: Some(id),
                platform: Platform::Upwork,
                external_id: format!("j{id}"),
                title: format!("Job {id}"),
                description: None,
                url: "https://e.com".into(),
                budget: None,
                tags: vec![],
                raw: Data::Upwork {
                    detail: UpworkJobDetail::default(),
                },
                created_at: Utc::now(),
                updated_at: Utc::now(),
                liked: None,
                note: None,
                applied_at,
            }
        }

        let jobs = vec![
            job(1, Some(Utc::now() - chrono::Duration::days(1))),
            job(2, None),
            job(3, Some(Utc::now() - chrono::Duration::days(5))),
        ];

        let ids = |f: JobFilter| {
            f.apply(jobs.clone())
                .into_iter()
                .map(|j| j.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(
            ids(JobFilter {
                applied: Some(true),
                ..Default::default()
            }),
            vec![Some(1), Some(3)]
        );
        assert_eq!(
            ids(JobFilter {
                applied: Some(false),
                ..Default::default()
            }),
            vec![Some(2)]
        );
        assert_eq!(
            ids(JobFilter {
                applied: None,
                ..Default::default()
            }),
            vec![Some(1), Some(2), Some(3)]
        );
    }
}
