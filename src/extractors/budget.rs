use crate::models::Budget;
use regex::Regex;
use std::sync::LazyLock;

fn normalize(s: &str) -> String {
    s.replace(['\u{00a0}', '\u{2007}', '\u{202f}', ',', '\t'], " ")
}

fn parse_number(s: &str) -> Option<u32> {
    let s = normalize(s);
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let last = s.chars().last()?;
    let multiplier: u64 = if last.eq_ignore_ascii_case(&'k') {
        1_000
    } else if last.eq_ignore_ascii_case(&'m') {
        1_000_000
    } else {
        1
    };
    let digits: String = s.chars().filter(char::is_ascii_digit).collect();
    let n: u64 = digits.parse().ok()?;
    (n * multiplier).try_into().ok()
}

/// Upwork budget strings: "Hourly: $50-$100/hr", "Fixed-price: $5,000", "$125 - $200/hr".
pub fn parse_upwork_budget(s: &str) -> Option<Budget> {
    let lower = s.to_lowercase();
    let period = if lower.contains("hour") || lower.contains("/hr") {
        Some("hr")
    } else {
        None
    };

    let cleaned = s
        .replace("Hourly:", "")
        .replace("hourly:", "")
        .replace("Fixed-price:", "")
        .replace("fixed-price:", "")
        .replace("Fixed price:", "")
        .replace("fixed price:", "");
    let cleaned = normalize(&cleaned);

    static RANGE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\$?\s*(\d[\d\s,]*[kKmM]?)\s*(?:–|-|\s+to\s+|and)\s*\$?\s*(\d[\d\s,]*[kKmM]?)\s*(?:/hr|hour|hours)?").unwrap()
    });
    static SINGLE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\$?\s*(\d[\d\s,]*[kKmM]?)\s*(?:/hr|hour|hours)?").unwrap()
    });

    if let Some(caps) = RANGE_RE.captures(&cleaned) {
        let min = parse_number(&caps[1])?;
        let max = parse_number(&caps[2])?;
        return Some(if min == max {
            Budget::Single {
                amount: min,
                currency: "USD".to_string(),
                period: period.map(std::string::ToString::to_string),
            }
        } else {
            Budget::Range {
                min,
                max,
                currency: "USD".to_string(),
                period: period.map(std::string::ToString::to_string),
            }
        });
    }

    let caps = SINGLE_RE.captures(&cleaned)?;
    let amount = parse_number(&caps[1])?;
    Some(Budget::Single {
        amount,
        currency: "USD".to_string(),
        period: period.map(std::string::ToString::to_string),
    })
}

/// `NoFluffJobs` budget strings: "7 069 – 9 426 EUR", "15 000 – 20 000 PLN", "Salary Match".
pub fn parse_nofluff_budget(s: &str) -> Option<Budget> {
    let cleaned = normalize(s).trim().to_string();

    static RANGE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)(\d[\d\s]*)\s*[–-]\s*(\d[\d\s]*)\s*(EUR|PLN|USD|GBP|CHF)").unwrap()
    });
    static SINGLE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)(\d[\d\s]*)\s*(EUR|PLN|USD|GBP|CHF)").unwrap());

    if let Some(caps) = RANGE_RE.captures(&cleaned) {
        let min = parse_number(&caps[1])?;
        let max = parse_number(&caps[2])?;
        let currency = caps[3].to_ascii_uppercase();
        return Some(if min == max {
            Budget::Single {
                amount: min,
                currency,
                period: Some("mo".to_string()),
            }
        } else {
            Budget::Range {
                min,
                max,
                currency,
                period: Some("mo".to_string()),
            }
        });
    }

    let caps = SINGLE_RE.captures(&cleaned)?;
    let amount = parse_number(&caps[1])?;
    let currency = caps[2].to_ascii_uppercase();
    Some(Budget::Single {
        amount,
        currency,
        period: Some("mo".to_string()),
    })
}

/// eFinancialCareers budget strings: "USD120000 - USD140000 per annum".
pub fn parse_efinancialcareers_budget(s: &str) -> Option<Budget> {
    let cleaned = normalize(s);

    static RANGE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\b(USD|EUR|GBP|PLN|CHF)\s*(\d[\d,]*[kKmM]?)\s*-\s*(?:USD|EUR|GBP|PLN|CHF)\s*(\d[\d,]*[kKmM]?)\b").unwrap()
    });
    static SINGLE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\b(USD|EUR|GBP|PLN|CHF)\s*(\d[\d,]*[kKmM]?)\b").unwrap());

    if let Some(caps) = RANGE_RE.captures(&cleaned) {
        let min = parse_number(&caps[2])?;
        let max = parse_number(&caps[3])?;
        let currency = caps[1].to_ascii_uppercase();
        return Some(if min == max {
            Budget::Single {
                amount: min,
                currency,
                period: Some("year".to_string()),
            }
        } else {
            Budget::Range {
                min,
                max,
                currency,
                period: Some("year".to_string()),
            }
        });
    }

    let caps = SINGLE_RE.captures(&cleaned)?;
    let currency = caps[1].to_ascii_uppercase();
    let amount = parse_number(&caps[2])?;
    Some(Budget::Single {
        amount,
        currency,
        period: Some("year".to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upwork_hourly_range() {
        let b = parse_upwork_budget("$50-$100/hr").unwrap();
        assert_eq!(b.to_string(), "50 - 100 USD/hr");
    }

    #[test]
    fn test_upwork_fixed() {
        let b = parse_upwork_budget("$5,000").unwrap();
        assert_eq!(b.to_string(), "5000 USD");
    }

    #[test]
    fn test_upwork_hourly_with_prefix() {
        let b = parse_upwork_budget("Hourly: $125 - $200/hr").unwrap();
        assert_eq!(b.to_string(), "125 - 200 USD/hr");
    }

    #[test]
    fn test_upwork_unknown_returns_none() {
        assert!(parse_upwork_budget("Negotiable").is_none());
    }

    #[test]
    fn test_nofluff_range() {
        let b = parse_nofluff_budget("7 069 – 9 426 EUR").unwrap();
        assert_eq!(b.to_string(), "7069 - 9426 EUR/mo");
    }

    #[test]
    fn test_nofluff_nbsp() {
        let b = parse_nofluff_budget("7\u{00a0}069 – 9\u{00a0}426 EUR").unwrap();
        assert_eq!(b.to_string(), "7069 - 9426 EUR/mo");
    }

    #[test]
    fn test_nofluff_pln() {
        let b = parse_nofluff_budget("15 000 – 20 000 PLN").unwrap();
        assert_eq!(b.to_string(), "15000 - 20000 PLN/mo");
    }

    #[test]
    fn test_nofluff_salary_match_returns_none() {
        assert!(parse_nofluff_budget("Salary Match").is_none());
    }

    #[test]
    fn test_efinancialcareers_usd_per_annum() {
        let b = parse_efinancialcareers_budget("USD120000 - USD140000 per annum").unwrap();
        assert_eq!(b.to_string(), "120000 - 140000 USD/year");
    }

    #[test]
    fn test_efinancialcareers_gbp_single() {
        let b = parse_efinancialcareers_budget("GBP90000 per annum").unwrap();
        assert_eq!(b.to_string(), "90000 GBP/year");
    }
}
