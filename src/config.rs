//! Application configuration loaded from `jobsearch.toml`.

// TODO(phase1): Define `Settings` struct with fields:
//   - `location: String`
//   - `pause_ms: u64`
//   - `providers: Providers`
// TODO(phase1): Define `ProviderConfig` struct with fields:
//   - `urls: Vec<String>`
//   - `pause_ms: Option<u64>`
// TODO(phase1): Define `Providers` struct with fields:
//   - `upwork: ProviderConfig`
//   - `nofluffjobs: ProviderConfig`
//   - `efinancialcareers: ProviderConfig`
//   - `linkedin: ProviderConfig`
// TODO(phase1): Implement `Settings::load(path: &std::path::Path) -> Result<Self, anyhow::Error>`.
// TODO(phase1): Implement `Settings::sample() -> Self` or a sample TOML string for `init` command.
