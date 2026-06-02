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

## Commands

All commands support `--json` for machine-readable output.

### Update / fetch jobs

```bash
./target/release/jobsearch update --platform nofluffjobs --query "rust"
./target/release/jobsearch update --platform upwork --query "rust"
./target/release/jobsearch update --query "rust"          # both platforms
```

### List jobs

```bash
./target/release/jobsearch list --status new --limit 20 --json
./target/release/jobsearch list --platform upwork --json
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

### Stats

```bash
./target/release/jobsearch stats --json
```

## Workflow guidance

1. Run `update` to fetch fresh jobs
2. List `new` jobs with `--json`
3. For interesting jobs, run `show <id>` to get full details
4. Use `react <id> save` to bookmark, `hide` to filter out noise
5. Periodically check `stats` for pipeline overview

## Database

SQLite at `~/.local/share/jobsearch/jobsearch.db`. Jobs are deduplicated by `(platform, external_id)`.
