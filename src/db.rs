use crate::models::{Data, Job, JobStatus, Platform, Reaction};
use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use std::str::FromStr;

pub struct Db {
    pool: SqlitePool,
}

impl Db {
    pub async fn open(path: &std::path::Path) -> Result<Self> {
        let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", path.display()))?
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn upsert_job(&self, job: &Job) -> Result<i64> {
        let tags = serde_json::to_string(&job.tags)?;
        let raw = serde_json::to_string(&job.raw)?;
        let platform = job.platform.to_string();
        let status = job.status.to_string();

        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO jobs (platform, external_id, title, description, url, posted_at, budget, tags, raw, status)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(platform, external_id) DO UPDATE SET
                title = excluded.title,
                description = excluded.description,
                url = excluded.url,
                posted_at = excluded.posted_at,
                budget = excluded.budget,
                tags = excluded.tags,
                raw = excluded.raw,
                updated_at = CURRENT_TIMESTAMP
            RETURNING id
            "#,
            platform,
            job.external_id,
            job.title,
            job.description,
            job.url,
            job.posted_at,
            job.budget,
            tags,
            raw,
            status,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    pub async fn list_jobs(
        &self,
        platform: Option<Platform>,
        status: Option<JobStatus>,
        limit: i64,
    ) -> Result<Vec<Job>> {
        let platform = platform.map(|p| p.to_string());
        let status = status.map(|s| s.to_string());

        let rows = sqlx::query_as!(
            JobRow,
            r#"
            SELECT id, platform, external_id, title, description, url, posted_at, budget, tags, raw, status, created_at, updated_at
            FROM jobs
            WHERE (?1 IS NULL OR platform = ?1) AND (?2 IS NULL OR status = ?2)
            ORDER BY posted_at DESC LIMIT ?3
            "#,
            platform,
            status,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn get_job(&self, id: i64) -> Result<Option<Job>> {
        let row = sqlx::query_as!(
            JobRow,
            r#"
            SELECT id, platform, external_id, title, description, url, posted_at, budget, tags, raw, status, created_at, updated_at
            FROM jobs WHERE id = ?1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn update_status(&self, id: i64, status: JobStatus) -> Result<()> {
        let status = status.to_string();
        sqlx::query!(
            "UPDATE jobs SET status = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            status,
            id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn add_reaction(
        &self,
        job_id: i64,
        action: Reaction,
        metadata: Option<String>,
    ) -> Result<()> {
        let action = action.to_string();
        sqlx::query!(
            "INSERT INTO reactions (job_id, action, metadata) VALUES (?1, ?2, ?3)",
            job_id,
            action,
            metadata
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_raw(&self, id: i64, raw: &Data) -> Result<()> {
        let raw_str = serde_json::to_string(raw)?;
        sqlx::query!(
            "UPDATE jobs SET raw = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            raw_str,
            id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Returns true if a job with this platform+external_id already exists.
    pub async fn job_exists(&self, platform: &Platform, external_id: &str) -> Result<bool> {
        let platform = platform.to_string();
        let count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM jobs WHERE platform = ?1 AND external_id = ?2",
            platform,
            external_id
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(count > 0)
    }

    pub async fn stats(&self) -> Result<Stats> {
        let total = sqlx::query_scalar!("SELECT COUNT(*) FROM jobs")
            .fetch_one(&self.pool)
            .await?;

        let by_status_rows =
            sqlx::query!("SELECT status, COUNT(*) as count FROM jobs GROUP BY status")
                .fetch_all(&self.pool)
                .await?;
        let by_status: Vec<(String, i64)> = by_status_rows
            .into_iter()
            .map(|r| (r.status, r.count))
            .collect();

        let by_platform_rows =
            sqlx::query!("SELECT platform, COUNT(*) as count FROM jobs GROUP BY platform")
                .fetch_all(&self.pool)
                .await?;
        let by_platform: Vec<(String, i64)> = by_platform_rows
            .into_iter()
            .map(|r| (r.platform, r.count))
            .collect();

        let new_count = sqlx::query_scalar!("SELECT COUNT(*) FROM jobs WHERE status = 'new'")
            .fetch_one(&self.pool)
            .await?;

        Ok(Stats {
            total,
            new_count,
            by_status,
            by_platform,
        })
    }
}

#[derive(sqlx::FromRow)]
struct JobRow {
    id: i64,
    platform: String,
    external_id: String,
    title: String,
    description: Option<String>,
    url: String,
    posted_at: Option<chrono::NaiveDateTime>,
    budget: Option<String>,
    tags: String,
    raw: String,
    status: String,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
}

impl From<JobRow> for Job {
    fn from(r: JobRow) -> Self {
        // Deserialize raw; fall back to default if old schema (missing `platform` tag)
        let raw: Data = serde_json::from_str(&r.raw).unwrap_or_else(|_| {
            let platform = match r.platform.as_str() {
                "upwork" => Platform::Upwork,
                _ => Platform::NoFluffJobs,
            };
            match platform {
                Platform::Upwork => Data::Upwork {
                    detail: crate::models::UpworkJobDetail::default(),
                },
                Platform::NoFluffJobs => Data::Nofluffjobs {
                    detail: crate::models::NoFluffJobDetail::default(),
                },
            }
        });

        Job {
            id: Some(r.id),
            platform: match r.platform.as_str() {
                "upwork" => Platform::Upwork,
                _ => Platform::NoFluffJobs,
            },
            external_id: r.external_id,
            title: r.title,
            description: r.description,
            url: r.url,
            posted_at: r.posted_at.map(|dt| dt.and_utc()),
            budget: r.budget,
            tags: serde_json::from_str(&r.tags).unwrap_or_default(),
            raw,
            status: match r.status.as_str() {
                "viewed" => JobStatus::Viewed,
                "saved" => JobStatus::Saved,
                "applied" => JobStatus::Applied,
                "rejected" => JobStatus::Rejected,
                "hidden" => JobStatus::Hidden,
                _ => JobStatus::New,
            },
            created_at: Some(r.created_at.and_utc()),
            updated_at: Some(r.updated_at.and_utc()),
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct Stats {
    pub total: i64,
    pub new_count: i64,
    pub by_status: Vec<(String, i64)>,
    pub by_platform: Vec<(String, i64)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Job, JobStatus, NoFluffJobDetail, Platform, UpworkJobDetail};

    fn temp_db() -> tempfile::NamedTempFile {
        tempfile::NamedTempFile::new().expect("temp db")
    }

    fn test_job(platform: Platform, external_id: &str, title: &str, status: JobStatus) -> Job {
        let raw = match platform {
            Platform::Upwork => Data::Upwork {
                detail: UpworkJobDetail::default(),
            },
            Platform::NoFluffJobs => Data::Nofluffjobs {
                detail: NoFluffJobDetail::default(),
            },
        };
        Job {
            id: None,
            platform,
            external_id: external_id.to_string(),
            title: title.to_string(),
            description: None,
            url: format!("https://example.com/{}", external_id),
            posted_at: None,
            budget: None,
            tags: vec![],
            raw,
            status,
            created_at: None,
            updated_at: None,
        }
    }

    #[tokio::test]
    async fn test_upsert_and_list() -> Result<()> {
        let tmp = temp_db();
        let db = Db::open(tmp.path()).await?;

        let job = test_job(Platform::Upwork, "abc123", "Rust Dev", JobStatus::New);
        let id = db.upsert_job(&job).await?;
        assert!(id > 0);

        let jobs = db.list_jobs(None, None, 10).await?;
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].title, "Rust Dev");
        Ok(())
    }

    #[tokio::test]
    async fn test_raw_roundtrip_upwork() -> Result<()> {
        let tmp = temp_db();
        let db = Db::open(tmp.path()).await?;

        let detail = UpworkJobDetail {
            proposals: "5 to 10".to_string(),
            last_viewed: "2 hours ago".to_string(),
            interviewing: "1".to_string(),
            invites_sent: "3".to_string(),
            unanswered_invites: "0".to_string(),
            description: "Build a Rust API".to_string(),
            exact_budget: "$50-$100/hr".to_string(),
            experience_level: "Expert".to_string(),
            hires: "0".to_string(),
            project_type: "Ongoing project".to_string(),
            duration: "1 to 3 months".to_string(),
            hours_per_week: "Less than 30 hrs/week".to_string(),
        };
        let job = Job {
            id: None,
            platform: Platform::Upwork,
            external_id: "uw-99".to_string(),
            title: "Rust Backend".to_string(),
            description: None,
            url: "https://upwork.com/jobs/uw-99".to_string(),
            posted_at: None,
            budget: Some("$5000".to_string()),
            tags: vec!["rust".to_string()],
            raw: Data::Upwork {
                detail: detail.clone(),
            },
            status: JobStatus::New,
            created_at: None,
            updated_at: None,
        };

        let id = db.upsert_job(&job).await?;
        let found = db.get_job(id).await?.expect("job exists");

        assert!(matches!(found.raw, Data::Upwork { .. }));
        if let Data::Upwork { detail: d } = found.raw {
            assert_eq!(d.proposals, detail.proposals);
            assert_eq!(d.exact_budget, detail.exact_budget);
            assert_eq!(d.experience_level, detail.experience_level);
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_raw_roundtrip_nofluffjobs() -> Result<()> {
        let tmp = temp_db();
        let db = Db::open(tmp.path()).await?;

        let detail = NoFluffJobDetail {
            company: "Acme Corp".to_string(),
            seniority: "Senior".to_string(),
            remote: "Fully remote".to_string(),
            locations: vec!["Warsaw".to_string(), "Berlin".to_string()],
            must_have: vec!["rust".to_string(), "docker".to_string()],
            requirements: "5+ years Rust".to_string(),
            offer_description: "Cool project".to_string(),
            offer_valid_until: "2026-12-31".to_string(),
            languages: vec!["en".to_string()],
        };
        let job = Job {
            id: None,
            platform: Platform::NoFluffJobs,
            external_id: "nf-88".to_string(),
            title: "Senior Rust".to_string(),
            description: None,
            url: "https://nofluffjobs.com/job/nf-88".to_string(),
            posted_at: None,
            budget: Some("8000 EUR".to_string()),
            tags: vec!["rust".to_string(), "remote".to_string()],
            raw: Data::Nofluffjobs {
                detail: detail.clone(),
            },
            status: JobStatus::New,
            created_at: None,
            updated_at: None,
        };

        let id = db.upsert_job(&job).await?;
        let found = db.get_job(id).await?.expect("job exists");

        assert!(matches!(found.raw, Data::Nofluffjobs { .. }));
        if let Data::Nofluffjobs { detail: d } = found.raw {
            assert_eq!(d.company, detail.company);
            assert_eq!(d.remote, detail.remote);
            assert_eq!(d.locations, detail.locations);
            assert_eq!(d.must_have, detail.must_have);
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_update_status() -> Result<()> {
        let tmp = temp_db();
        let db = Db::open(tmp.path()).await?;

        let job = test_job(Platform::NoFluffJobs, "nf1", "Backend", JobStatus::New);
        let id = db.upsert_job(&job).await?;

        db.update_status(id, JobStatus::Saved).await?;
        let found = db.get_job(id).await?.expect("job exists");
        assert_eq!(found.status, JobStatus::Saved);
        Ok(())
    }

    #[tokio::test]
    async fn test_stats() -> Result<()> {
        let tmp = temp_db();
        let db = Db::open(tmp.path()).await?;

        db.upsert_job(&test_job(Platform::Upwork, "u1", "A", JobStatus::New))
            .await?;
        db.upsert_job(&test_job(Platform::Upwork, "u2", "B", JobStatus::Saved))
            .await?;
        db.upsert_job(&test_job(Platform::NoFluffJobs, "n1", "C", JobStatus::New))
            .await?;

        let stats = db.stats().await?;
        assert_eq!(stats.total, 3);
        assert_eq!(stats.new_count, 2);
        Ok(())
    }
}
