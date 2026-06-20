---
name: jobsearch
description: Unified job search across Upwork and NoFluffJobs. Fetch, list, filter, and track job applications via local Rust CLI. Use when user wants to find jobs, track applications, or manage their job search pipeline.
---

# Job Search

Local CLI tool at `~/Documents/projects/job_search/target/release/jobsearch` (or just `jobsearch` if in PATH).

## Prerequisites

- **NoFluffJobs**: works out of the box (HTTP API)
- **Upwork**: requires Brave browser running with remote debugging. Start with:
  ```bash
  open -a "Brave Browser" --args --remote-debugging-port=9222
  ```
  Must be logged into Upwork in that Brave instance.
- **eFinancialCareers**: requires Brave browser running with remote debugging, and an open `efinancialcareers.com` tab (logged in for applications sync). Start with the command above and open the site.
- **Hacker News**: works out of the box via Algolia API (no browser). Fetches top-level job posts from the latest monthly "Ask HN: Who is hiring?" thread.

## Commands

All commands support `--json` for machine-readable output.

### Update / fetch jobs

```bash
./target/release/jobsearch update nofluff --query "rust"
./target/release/jobsearch update upwork --query "rust"
./target/release/jobsearch update efinancialcareers --query "Rust,Developer"
./target/release/jobsearch update hackernews --query "rust"
./target/release/jobsearch update --query "rust"          # all platforms
```

### List jobs

```bash
./target/release/jobsearch list --status new --limit 20 --json
./target/release/jobsearch list --platform upwork --json
./target/release/jobsearch list hackernews --json
```

Statuses: `new`, `viewed`, `saved`, `applied`, `rejected`, `hidden`

### Show job details

```bash
./target/release/jobsearch show <id> --json
```

### React to job

```bash
./target/release/jobsearch react <id> save
./target/release/jobsearch react <id> hide
./target/release/jobsearch react <id> apply
```

### Sync applications

```bash
./target/release/jobsearch sync-applications upwork
./target/release/jobsearch sync-applications nofluff
./target/release/jobsearch sync-applications efinancialcareers
```

### Sync likes between databases

Copies non-null liked/disliked state from source DB to target DB, matching by
`(platform, external_id)`. Rows missing in target are ignored.

```bash
./target/release/jobsearch sync-likes /path/to/source.db /path/to/target.db
```

### Stats

```bash
./target/release/jobsearch stats --json
```

## Workflow guidance

1. Run `update` to fetch fresh jobs
2. List `new` jobs with `--json`
3. For interesting jobs, run `show <id>` to get full details
4. Use `react <id> save` to bookmark, `hide` to filter out noise
5. For Hacker News jobs, mark applied manually with `react <id> apply` (no application sync available)
6. Periodically check `stats` for pipeline overview

## Database

SQLite at `~/.local/share/jobsearch/jobsearch.db`. Jobs are deduplicated by `(platform, external_id)`.
