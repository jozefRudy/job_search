use crate::models::{Job, JobStatus, Platform, Reaction};
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

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS jobs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                platform TEXT NOT NULL,
                external_id TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                url TEXT NOT NULL,
                posted_at TIMESTAMP,
                budget TEXT,
                tags TEXT,
                raw TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'new',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(platform, external_id)
            );

            CREATE TABLE IF NOT EXISTS reactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                job_id INTEGER NOT NULL REFERENCES jobs(id),
                action TEXT NOT NULL,
                metadata TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_jobs_platform ON jobs(platform);
            CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status);
            CREATE INDEX IF NOT EXISTS idx_jobs_external ON jobs(platform, external_id);
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    pub async fn upsert_job(&self, job: &Job) -> Result<i64> {
        let tags = serde_json::to_string(&job.tags)?;
        let raw = serde_json::to_string(&job.raw)?;

        let id: i64 = sqlx::query_scalar(
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
            "#
        )
        .bind(job.platform.to_string())
        .bind(&job.external_id)
        .bind(&job.title)
        .bind(&job.description)
        .bind(&job.url)
        .bind(job.posted_at)
        .bind(&job.budget)
        .bind(&tags)
        .bind(&raw)
        .bind(job.status.to_string())
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
        let mut query = String::from(
            "SELECT id, platform, external_id, title, description, url, posted_at, budget, tags, raw, status, created_at, updated_at FROM jobs WHERE 1=1",
        );

        if platform.is_some() {
            query.push_str(" AND platform = ?");
        }
        if status.is_some() {
            query.push_str(" AND status = ?");
        }
        query.push_str(" ORDER BY posted_at DESC LIMIT ?");

        let mut q = sqlx::query_as::<_, JobRow>(&query);

        if let Some(p) = platform {
            q = q.bind(p.to_string());
        }
        if let Some(s) = status {
            q = q.bind(s.to_string());
        }
        q = q.bind(limit);

        let rows: Vec<JobRow> = q.fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn get_job(&self, id: i64) -> Result<Option<Job>> {
        let row: Option<JobRow> = sqlx::query_as(
            "SELECT id, platform, external_id, title, description, url, posted_at, budget, tags, raw, status, created_at, updated_at FROM jobs WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn update_status(&self, id: i64, status: JobStatus) -> Result<()> {
        sqlx::query("UPDATE jobs SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(status.to_string())
            .bind(id)
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
        sqlx::query("INSERT INTO reactions (job_id, action, metadata) VALUES (?, ?, ?)")
            .bind(job_id)
            .bind(action.to_string())
            .bind(metadata)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_raw(&self, id: i64, raw: serde_json::Value) -> Result<()> {
        let raw_str = serde_json::to_string(&raw)?;
        sqlx::query("UPDATE jobs SET raw = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(raw_str)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn stats(&self) -> Result<Stats> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs")
            .fetch_one(&self.pool)
            .await?;

        let by_status: Vec<(String, i64)> =
            sqlx::query_as("SELECT status, COUNT(*) FROM jobs GROUP BY status")
                .fetch_all(&self.pool)
                .await?;

        let by_platform: Vec<(String, i64)> =
            sqlx::query_as("SELECT platform, COUNT(*) FROM jobs GROUP BY platform")
                .fetch_all(&self.pool)
                .await?;

        let new_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs WHERE status = 'new'")
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
    posted_at: Option<chrono::DateTime<chrono::Utc>>,
    budget: Option<String>,
    tags: String,
    raw: String,
    status: String,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<JobRow> for Job {
    fn from(r: JobRow) -> Self {
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
            posted_at: r.posted_at,
            budget: r.budget,
            tags: serde_json::from_str(&r.tags).unwrap_or_default(),
            raw: serde_json::from_str(&r.raw).unwrap_or(serde_json::Value::Null),
            status: match r.status.as_str() {
                "viewed" => JobStatus::Viewed,
                "saved" => JobStatus::Saved,
                "applied" => JobStatus::Applied,
                "rejected" => JobStatus::Rejected,
                "hidden" => JobStatus::Hidden,
                _ => JobStatus::New,
            },
            created_at: r.created_at,
            updated_at: r.updated_at,
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
    use crate::models::{Job, JobStatus, Platform};

    fn temp_db() -> tempfile::NamedTempFile {
        tempfile::NamedTempFile::new().expect("temp db")
    }

    fn test_job(platform: Platform, external_id: &str, title: &str, status: JobStatus) -> Job {
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
            raw: serde_json::json!({}),
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
