//! TODO: `pub enum Region { Europe, NorthAmerica, SouthAmerica, Asia, Africa, Oceania, MiddleEast }`
//!   - `impl FromStr` (accepts "europe", "north-america"/"north_america" etc.; err lists
//!     valid options) + `impl Display` ("Europe", for LLM prompt contexts)
//!     + serde Deserialize via FromStr.
//! TODO: `pub fn slug(&self) -> &'static str` — kebab-case wellfound location slug:
//!   Europe -> "europe", NorthAmerica -> "north-america", ... (verified 1:1 with
//!   wellfound /role/l/<role>/<slug> URLs).
//! TODO: tests — FromStr roundtrip with Display, slug values.
