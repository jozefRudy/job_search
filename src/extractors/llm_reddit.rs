//! TODO(phase2): Reddit LLM extraction fields.
//!
//! Two structs, SAME field shape, different prompts:
//! - JobbitFields: r/jobbit posts — semi-structured titles
//!   (role @ company, location, salary often in title alone).
//!   is_job_ad required: "Hiring" flair still contains junk (meta posts,
//!   challenges, seeker referrals under "Hiring - Open" flair); [HIRING]
//!   title prefix not reliable.
//! - MegathreadFields: r/rust "Who's Hiring" comments — prose; prompt must
//!   reject job-SEEKER presentations (single thread for seekers + offerers;
//!   seekers reply under designated top-level comment) and mod/meta/deleted
//!   comments; extract only job offers.
//! Extractable::PROMPT is a per-struct const, hence two structs (no shared
//! abstraction — YAGNI).

// TODO(phase2): shared shape for both structs (derive Deserialize, JsonSchema, Default):
//   is_job_ad: bool, company: Option<String>, role: Option<String>,
//   location: Option<String>, remote: Option<bool>, budget: Option<String>,
//   tags: Vec<String>
//   + schemars descriptions per-struct (jobbit mentions [HIRING] title,
//     megathread mentions prose comment + seeker exclusion)

// TODO(phase2): pub struct JobbitFields — impl Extractable
//   const PROMPT: PromptKind = PromptKind::RedditJobbit;
//   const HEALTHCHECK_TEXT: include_str!("llm/fixtures/reddit_jobbit_healthcheck.md");
//   fn verify() — is_job_ad, company contains "acme", role contains "rust", remote Some(true)

// TODO(phase2): pub struct MegathreadFields — impl Extractable
//   const PROMPT: PromptKind = PromptKind::RedditMegathread;
//   const HEALTHCHECK_TEXT: include_str!("llm/fixtures/reddit_megathread_healthcheck.md");
//   fn verify() — same expectations

// TODO(phase3): ignored integration tests mirroring llm_hackernews.rs tests
//   (#[ignore = "requires JOBSEARCH_LLM_CLI set to an LLM CLI command"];
//    fixture via include_str!("llm/fixtures/reddit_*.md"), .expect() not unwrap):
//   test_extract_jobbit_from_fixture — job ad: is_job_ad, company, role, remote,
//     budget parsed from [HIRING] title alone
//   test_extract_jobbit_rejects_non_job — fixture: meta/"I got the job!"-style post
//     => is_job_ad == false
//   test_extract_megathread_from_fixture — job ad from prose comment
//   test_extract_megathread_rejects_seeker — job-seeker comment fixture
//     => is_job_ad == false
//   test_extract_megathread_rejects_grant — grant-funding/open-call fixture
//     => is_job_ad == false (not employment)
