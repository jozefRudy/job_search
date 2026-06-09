use crate::models::{Data, Job, Paginated, Platform, Rating, Sort};
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
        let platform = &job.platform;
        let created_at = job.created_at.naive_utc();

        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO jobs (platform, external_id, title, description, url, budget, tags, raw, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(platform, external_id) DO UPDATE SET
                title = excluded.title,
                description = excluded.description,
                url = excluded.url,
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
            job.budget,
            tags,
            raw,
            created_at,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    pub async fn list_jobs(
        &self,
        platform: Option<Platform>,
        sort: Sort,
        limit: i64,
    ) -> Result<Vec<Job>> {
        let order_by = sort.order_by_sql();
        let sql = format!(
            r#"
            SELECT
                j.id, j.platform, j.external_id, j.title, j.description,
                j.url, j.budget, j.tags, j.raw, j.created_at, j.updated_at,
                j.liked, r.note, r.applied_at
            FROM jobs j
            LEFT JOIN reactions r ON r.job_id = j.id
            WHERE (?1 IS NULL OR j.platform = ?1)
            ORDER BY {} LIMIT ?2
            "#,
            order_by
        );
        let rows = sqlx::query_as::<_, JobRow>(&sql)
            .bind(platform)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn list_jobs_filtered(
        &self,
        platform: Option<Platform>,
        liked: Option<Rating>,
        sort: Sort,
        limit: i64,
        offset: i64,
    ) -> Result<Paginated<Job>> {
        let order_by = sort.order_by_sql();
        let liked_str = liked.as_ref().map(|r| match r {
            Rating::Liked => "liked",
            Rating::Disliked => "disliked",
            Rating::Neutral => "neutral",
        });

        let total: i64 = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*)
            FROM jobs j
            LEFT JOIN reactions r ON r.job_id = j.id
            WHERE (?1 IS NULL OR j.platform = ?1)
              AND (?2 IS NULL OR (
                (?2 = 'liked' AND j.liked = 1) OR
                (?2 = 'disliked' AND j.liked = 0) OR
                (?2 = 'neutral' AND j.liked IS NULL)
              ))
            "#,
            platform,
            liked_str,
        )
        .fetch_one(&self.pool)
        .await?;
        let sql = format!(
            r#"
            SELECT
                j.id, j.platform, j.external_id, j.title, j.description,
                j.url, j.budget, j.tags, j.raw, j.created_at, j.updated_at,
                j.liked, r.note, r.applied_at
            FROM jobs j
            LEFT JOIN reactions r ON r.job_id = j.id
            WHERE (?1 IS NULL OR j.platform = ?1)
              AND (?2 IS NULL OR (
                (?2 = 'liked' AND j.liked = 1) OR
                (?2 = 'disliked' AND j.liked = 0) OR
                (?2 = 'neutral' AND j.liked IS NULL)
              ))
            ORDER BY {} LIMIT ?3 OFFSET ?4
            "#,
            order_by
        );
        let rows = sqlx::query_as::<_, JobRow>(&sql)
            .bind(platform)
            .bind(liked_str)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        let items = rows.into_iter().map(|r| r.into()).collect();
        Ok(Paginated { items, total })
    }

    pub async fn get_job(&self, id: i64) -> Result<Option<Job>> {
        let row = sqlx::query_as!(
            JobRow,
            r#"
            SELECT
                j.id, j.platform, j.external_id, j.title, j.description,
                j.url, j.budget, j.tags, j.raw, j.created_at, j.updated_at,
                j.liked, r.note, r.applied_at
            FROM jobs j
            LEFT JOIN reactions r ON r.job_id = j.id
            WHERE j.id = ?1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn set_applied(&self, job_id: i64, note: Option<&str>) -> Result<()> {
        sqlx::query!(
            "INSERT INTO reactions (job_id, note) VALUES (?1, ?2)
             ON CONFLICT(job_id) DO UPDATE SET note = excluded.note",
            job_id,
            note
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_liked(&self, ids: &[i64], liked: bool) -> Result<()> {
        let ids_json = serde_json::to_string(ids)?;
        sqlx::query!(
            "UPDATE jobs SET liked = ?1 WHERE id IN (SELECT value FROM json_each(?2))",
            liked,
            ids_json
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_neutral(&self, ids: &[i64]) -> Result<()> {
        let ids_json = serde_json::to_string(ids)?;
        sqlx::query!(
            "UPDATE jobs SET liked = NULL WHERE id IN (SELECT value FROM json_each(?1))",
            ids_json
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
        let count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM jobs WHERE platform = ?1 AND external_id = ?2",
            platform,
            external_id
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(count > 0)
    }

    /// Fetch updated_at for a job by platform + external_id.
    pub async fn job_updated_at(
        &self,
        platform: &Platform,
        external_id: &str,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>> {
        let row = sqlx::query_scalar!(
            "SELECT updated_at FROM jobs WHERE platform = ?1 AND external_id = ?2",
            platform,
            external_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|dt| dt.and_utc()))
    }

    pub async fn stats(&self) -> Result<Stats> {
        let total = sqlx::query_scalar!("SELECT COUNT(*) FROM jobs")
            .fetch_one(&self.pool)
            .await?;

        let by_platform_rows =
            sqlx::query!("SELECT platform, COUNT(*) as count FROM jobs GROUP BY platform")
                .fetch_all(&self.pool)
                .await?;
        let by_platform: Vec<(String, i64)> = by_platform_rows
            .into_iter()
            .map(|r| (r.platform, r.count))
            .collect();

        Ok(Stats { total, by_platform })
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
    budget: Option<String>,
    tags: String,
    raw: String,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
    liked: Option<bool>,
    note: Option<String>,
    applied_at: Option<chrono::NaiveDateTime>,
}

impl From<JobRow> for Job {
    fn from(r: JobRow) -> Self {
        // Deserialize raw; fall back to default if old schema (missing `platform` tag)
        let raw: Data =
            serde_json::from_str(&r.raw).unwrap_or_else(|_| match r.platform.as_str() {
                "upwork" => Data::Upwork {
                    detail: crate::models::UpworkJobDetail::default(),
                },
                "nofluffjobs" => Data::Nofluffjobs {
                    detail: crate::models::NoFluffJobDetail::default(),
                },
                _ => panic!("unknown platform in db: '{}'", r.platform),
            });

        Job {
            id: Some(r.id),
            platform: match r.platform.as_str() {
                "upwork" => Platform::Upwork,
                "nofluffjobs" => Platform::NoFluffJobs,
                _ => panic!("unknown platform in db: '{}'", r.platform),
            },
            external_id: r.external_id,
            title: r.title,
            description: r.description,
            url: r.url,
            budget: r.budget,
            tags: serde_json::from_str(&r.tags).unwrap_or_default(),
            raw,
            created_at: r.created_at.and_utc(),
            updated_at: r.updated_at.and_utc(),
            note: r.note,
            liked: r.liked,
            applied_at: r.applied_at.map(|dt| dt.and_utc()),
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct Stats {
    pub total: i64,
    pub by_platform: Vec<(String, i64)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Job, NoFluffJobDetail, Platform, UpworkJobDetail};

    fn temp_db() -> tempfile::NamedTempFile {
        tempfile::NamedTempFile::new().expect("temp db")
    }

    fn test_job(platform: Platform, external_id: &str, title: &str) -> Job {
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
            budget: None,
            tags: vec![],
            raw,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            liked: None,
            note: None,
            applied_at: None,
        }
    }

    #[tokio::test]
    async fn test_upsert_and_list() -> Result<()> {
        let tmp = temp_db();
        let db = Db::open(tmp.path()).await?;

        let job = test_job(Platform::Upwork, "abc123", "Rust Dev");
        let id = db.upsert_job(&job).await?;
        assert!(id > 0);

        let jobs = db.list_jobs(None, Sort::Created, 10).await?;
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
            last_viewed: None,
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
            budget: Some("$5000".to_string()),
            tags: vec!["rust".to_string()],
            raw: Data::Upwork {
                detail: detail.clone(),
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            liked: None,
            note: None,
            applied_at: None,
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
            description: "Build backend".to_string(),
            requirements: "5+ years Rust".to_string(),
            nice_to_have: "Cool project".to_string(),
            offer_valid_until: "2026-12-31".to_string(),
            languages: vec!["en".to_string()],
            posted_at: None,
        };
        let job = Job {
            id: None,
            platform: Platform::NoFluffJobs,
            external_id: "nf-88".to_string(),
            title: "Senior Rust".to_string(),
            description: None,
            url: "https://nofluffjobs.com/job/nf-88".to_string(),
            budget: Some("8000 EUR".to_string()),
            tags: vec!["rust".to_string(), "remote".to_string()],
            raw: Data::Nofluffjobs {
                detail: detail.clone(),
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            liked: None,
            note: None,
            applied_at: None,
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
    async fn test_stats() -> Result<()> {
        let tmp = temp_db();
        let db = Db::open(tmp.path()).await?;

        db.upsert_job(&test_job(Platform::Upwork, "u1", "A"))
            .await?;
        db.upsert_job(&test_job(Platform::Upwork, "u2", "B"))
            .await?;
        db.upsert_job(&test_job(Platform::NoFluffJobs, "n1", "C"))
            .await?;

        let stats = db.stats().await?;
        assert_eq!(stats.total, 3);
        Ok(())
    }
}
