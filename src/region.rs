//! User region, shared by config, wellfound URL validation and LLM prompt contexts.

use anyhow::Result;
use serde::Deserialize;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    Europe,
    NorthAmerica,
    SouthAmerica,
    Asia,
    Africa,
    Oceania,
    MiddleEast,
}

impl Region {
    pub const ALL: [Region; 7] = [
        Region::Europe,
        Region::NorthAmerica,
        Region::SouthAmerica,
        Region::Asia,
        Region::Africa,
        Region::Oceania,
        Region::MiddleEast,
    ];
}

impl FromStr for Region {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let norm = s.trim().to_lowercase().replace(['_', ' '], "-");
        Region::ALL
            .into_iter()
            .find(|r| r.to_string().to_lowercase().replace(' ', "-") == norm)
            .ok_or_else(|| {
                let valid = Region::ALL
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                anyhow::anyhow!("unknown region {s:?}; valid: {valid}")
            })
    }
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Region::Europe => "Europe",
            Region::NorthAmerica => "North America",
            Region::SouthAmerica => "South America",
            Region::Asia => "Asia",
            Region::Africa => "Africa",
            Region::Oceania => "Oceania",
            Region::MiddleEast => "Middle East",
        };
        f.write_str(name)
    }
}

impl serde::Serialize for Region {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Region {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::Region;
    use std::str::FromStr;

    #[test]
    fn from_str_roundtrip() {
        for r in Region::ALL {
            assert_eq!(Region::from_str(&r.to_string()).expect("display parses"), r);
        }
        assert_eq!(Region::from_str(" europe ").expect("trims"), Region::Europe);
        assert_eq!(
            Region::from_str("north-america").expect("kebab"),
            Region::NorthAmerica
        );
        assert_eq!(
            Region::from_str("north_america").expect("underscore"),
            Region::NorthAmerica
        );
        assert!(Region::from_str("atlantis").is_err());
    }
}
