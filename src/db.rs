use crate::models::{Data, Job, JobFilter, Paginated, Platform, Rating, Sort};
use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use std::str::FromStr;

pub struct Db {
    pool: SqlitePool,
}

impl Db {
    pub async fn open(path: &std::path::Path) -> Result<Self> {
        let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", path.display()))?
            .create_if_missing(true)
            .foreign_keys(true);

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
            INSERT INTO jobs (platform, external_id, title, description, url, budget, tags, raw, created_at, remote)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(platform, external_id) DO UPDATE SET
                title = excluded.title,
                description = excluded.description,
                url = excluded.url,
                budget = excluded.budget,
                tags = excluded.tags,
                raw = excluded.raw,
                remote = excluded.remote,
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
            job.remote,
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
                j.url, j.budget, j.tags, j.raw, j.company, j.created_at, j.updated_at,
                j.liked, j.remote, r.note, r.applied_at
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
        filter: &JobFilter,
        sort: Sort,
        limit: i64,
        offset: i64,
    ) -> Result<Paginated<Job>> {
        let order_by = sort.order_by_sql();
        let liked_str = filter.liked.as_ref().map(|r| match r {
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
              AND (?3 IS NULL OR (
                (?3 = 1 AND r.applied_at IS NOT NULL) OR
                (?3 = 0 AND r.applied_at IS NULL)
              ))
              AND (?4 IS NULL OR j.remote = ?4)
            "#,
            filter.platform,
            liked_str,
            filter.applied,
            filter.remote,
        )
        .fetch_one(&self.pool)
        .await?;
        let sql = format!(
            r#"
            SELECT
                j.id, j.platform, j.external_id, j.title, j.description,
                j.url, j.budget, j.tags, j.raw, j.company, j.created_at, j.updated_at,
                j.liked, j.remote, r.note, r.applied_at
            FROM jobs j
            LEFT JOIN reactions r ON r.job_id = j.id
            WHERE (?1 IS NULL OR j.platform = ?1)
              AND (?2 IS NULL OR (
                (?2 = 'liked' AND j.liked = 1) OR
                (?2 = 'disliked' AND j.liked = 0) OR
                (?2 = 'neutral' AND j.liked IS NULL)
              ))
              AND (?3 IS NULL OR (
                (?3 = 1 AND r.applied_at IS NOT NULL) OR
                (?3 = 0 AND r.applied_at IS NULL)
              ))
              AND (?4 IS NULL OR j.remote = ?4)
            ORDER BY {} LIMIT ?5 OFFSET ?6
            "#,
            order_by
        );
        let rows = sqlx::query_as::<_, JobRow>(&sql)
            .bind(filter.platform)
            .bind(liked_str)
            .bind(filter.applied)
            .bind(filter.remote)
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
                j.url, j.budget, j.tags, j.raw, j.company, j.created_at, j.updated_at,
                j.liked, j.remote, r.note, r.applied_at
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

    pub async fn set_applied(
        &self,
        job_id: i64,
        note: Option<&str>,
        applied_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        let applied_at_naive = applied_at.naive_utc();
        sqlx::query!(
            "INSERT INTO reactions (job_id, note, applied_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(job_id) DO UPDATE SET
                note = excluded.note,
                applied_at = excluded.applied_at",
            job_id,
            note,
            applied_at_naive
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn unset_applied(&self, job_id: i64) -> Result<()> {
        sqlx::query!("DELETE FROM reactions WHERE job_id = ?1", job_id)
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

    /// Copy non-null liked state from `source_path` into self, matching by
    /// (platform, external_id). Ignore rows missing in target.
    pub async fn sync_likes(&self, source_path: &str) -> Result<u64> {
        let mut conn = self.pool.acquire().await?;

        sqlx::query("ATTACH DATABASE ? AS source")
            .bind(source_path)
            .execute(&mut *conn)
            .await?;

        let result = sqlx::query(
            r#"
            UPDATE jobs
            SET liked = s.liked
            FROM source.jobs AS s
            WHERE jobs.platform = s.platform
              AND jobs.external_id = s.external_id
              AND s.liked IS NOT NULL
            "#,
        )
        .execute(&mut *conn)
        .await;

        let _ = sqlx::query("DETACH DATABASE source")
            .execute(&mut *conn)
            .await;

        Ok(result?.rows_affected())
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

    /// Fetch job id by platform + external_id.
    pub async fn find_job_id(&self, platform: &Platform, external_id: &str) -> Result<Option<i64>> {
        let id = sqlx::query_scalar!(
            "SELECT id FROM jobs WHERE platform = ?1 AND external_id = ?2",
            platform,
            external_id
        )
        .fetch_optional(&self.pool)
        .await?
        .flatten();
        Ok(id)
    }

    pub async fn filter_new(&self, platform: &Platform, ids: &[String]) -> Result<Vec<String>> {
        let ids_json = serde_json::to_string(ids)?;
        let platform = platform.to_string();
        let pending: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT value
            FROM json_each(?1)
            WHERE value NOT IN (SELECT external_id FROM jobs WHERE platform = ?2)
              AND value NOT IN (SELECT external_id FROM rejected_jobs WHERE platform = ?2)
            "#,
        )
        .bind(&ids_json)
        .bind(&platform)
        .fetch_all(&self.pool)
        .await?;
        Ok(pending)
    }

    pub async fn mark_rejected(
        &self,
        platform: &Platform,
        external_id: &str,
        reason: &str,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO rejected_jobs (platform, external_id, reason)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(platform, external_id) DO UPDATE SET
                reason = excluded.reason,
                rejected_at = CURRENT_TIMESTAMP
            "#,
            platform,
            external_id,
            reason,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Check whether a Hacker News job post with the same company and role
    /// already exists with a `created_at` later than `since`.
    pub async fn has_similar_hackernews_post(
        &self,
        company: &str,
        role: &str,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<bool> {
        let cutoff = since.naive_utc();
        let company = Some(company);
        let count: i64 = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*)
            FROM jobs
            WHERE platform = 'hackernews'
              AND company = ?1
              AND json_extract(raw, '$.detail.role') = ?2
              AND created_at > ?3
            "#,
            company,
            role,
            cutoff,
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

    pub async fn delete_jobs(&self, ids: &[i64]) -> Result<u64> {
        let ids_json = serde_json::to_string(ids)?;
        let rows = sqlx::query!(
            "DELETE FROM jobs WHERE id IN (SELECT value FROM json_each(?1))",
            ids_json
        )
        .execute(&self.pool)
        .await?;
        Ok(rows.rows_affected())
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
    company: Option<String>,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
    liked: Option<bool>,
    remote: bool,
    note: Option<String>,
    applied_at: Option<chrono::NaiveDateTime>,
}

impl From<JobRow> for Job {
    fn from(r: JobRow) -> Self {
        let raw: Data = serde_json::from_str(&r.raw)
            .unwrap_or_else(|e| panic!("failed to deserialize raw for job {}: {}", r.id, e));

        Job {
            id: r.id,
            platform: r.platform.parse().expect("unknown platform in db"),
            external_id: r.external_id,
            title: r.title,
            description: r.description,
            url: r.url,
            budget: r.budget,
            tags: serde_json::from_str(&r.tags).unwrap_or_default(),
            raw,
            company: r.company,
            created_at: r.created_at.and_utc(),
            updated_at: r.updated_at.and_utc(),
            note: r.note,
            liked: r.liked,
            remote: r.remote,
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
    use crate::models::{
        EfinancialcareersJobDetail, Job, NoFluffJobDetail, Platform, UpworkJobDetail,
    };

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
            Platform::Efinancialcareers => Data::Efinancialcareers {
                detail: EfinancialcareersJobDetail::default(),
            },
            Platform::Hackernews => Data::Hackernews {
                detail: crate::models::HackerNewsJobDetail::default(),
            },
        };
        Job {
            id: 0,
            platform,
            external_id: external_id.to_string(),
            title: title.to_string(),
            description: None,
            url: format!("https://example.com/{}", external_id),
            budget: None,
            tags: vec![],
            raw,
            company: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            liked: None,
            note: None,
            applied_at: None,
            remote: true,
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
            tags: vec!["rust".to_string(), "api".to_string()],
            posted_at: chrono::Utc::now(),
        };
        let job = Job {
            id: 0,
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
            company: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            liked: None,
            note: None,
            applied_at: None,
            remote: true,
        };

        let id = db.upsert_job(&job).await?;
        let found = db.get_job(id).await?.expect("job exists");

        assert!(matches!(found.raw, Data::Upwork { .. }));
        if let Data::Upwork { detail: d } = found.raw {
            assert_eq!(d.proposals, detail.proposals);
            assert_eq!(d.exact_budget, detail.exact_budget);
            assert_eq!(d.experience_level, detail.experience_level);
            assert_eq!(d.tags, detail.tags);
            assert_eq!(d.posted_at, detail.posted_at);
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
            locations: vec!["Warsaw".to_string(), "Berlin".to_string()],
            must_have: vec!["rust".to_string(), "docker".to_string()],
            description: "Build backend".to_string(),
            requirements: "5+ years Rust".to_string(),
            nice_to_have: "Cool project".to_string(),
            offer_valid_until: "2026-12-31".to_string(),
            languages: vec!["en".to_string()],
            posted_at: chrono::Utc::now(),
            employment_type: Some("b2b".to_string()),
        };
        let job = Job {
            id: 0,
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
            company: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            liked: None,
            note: None,
            applied_at: None,
            remote: true,
        };

        let id = db.upsert_job(&job).await?;
        let found = db.get_job(id).await?.expect("job exists");

        assert!(matches!(found.raw, Data::Nofluffjobs { .. }));
        if let Data::Nofluffjobs { detail: d } = found.raw {
            assert_eq!(d.company, detail.company);
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

    #[tokio::test]
    async fn test_delete_job_cascades_to_reactions() -> Result<()> {
        let tmp = temp_db();
        let db = Db::open(tmp.path()).await?;

        let job = test_job(Platform::Upwork, "del-cascade", "Delete me");
        let id = db.upsert_job(&job).await?;
        db.set_applied(id, Some("note"), chrono::Utc::now()).await?;

        let before = db.get_job(id).await?;
        assert!(before.is_some());

        let deleted = db.delete_jobs(&[id]).await?;
        assert_eq!(deleted, 1);

        let after = db.get_job(id).await?;
        assert!(after.is_none());

        let reaction_count: i64 =
            sqlx::query_scalar!("SELECT COUNT(*) FROM reactions WHERE job_id = ?1", id)
                .fetch_one(&db.pool)
                .await?;
        assert_eq!(reaction_count, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_sync_likes_updates_existing_ignores_missing() -> Result<()> {
        let source_tmp = temp_db();
        let target_tmp = temp_db();
        let source = Db::open(source_tmp.path()).await?;
        let target = Db::open(target_tmp.path()).await?;

        let a = test_job(Platform::Upwork, "a", "A");
        let b = test_job(Platform::Upwork, "b", "B");
        let c = test_job(Platform::Upwork, "c", "C");
        let d = test_job(Platform::NoFluffJobs, "d", "D");

        let id_a = source.upsert_job(&a).await?;
        let id_b = source.upsert_job(&b).await?;
        let _id_c = source.upsert_job(&c).await?;
        let _id_d = source.upsert_job(&d).await?;
        source.set_liked(&[id_a], true).await?;
        source.set_liked(&[id_b], false).await?;

        // Target has A, B, C; lacks D.
        let target_a = target.upsert_job(&a).await?;
        let target_b = target.upsert_job(&b).await?;
        target.upsert_job(&c).await?;
        target.set_liked(&[target_a, target_b], true).await?;

        let synced = target
            .sync_likes(
                source_tmp
                    .path()
                    .to_str()
                    .ok_or(anyhow::anyhow!("invalid temp path"))?,
            )
            .await?;
        assert_eq!(synced, 2);

        let jobs = target.list_jobs(None, Sort::Created, 100).await?;
        let find = |ext_id: &str| jobs.iter().find(|j| j.external_id == ext_id).unwrap();
        assert_eq!(find("a").liked, Some(true));
        assert_eq!(find("b").liked, Some(false));
        assert_eq!(find("c").liked, None);

        Ok(())
    }

    #[tokio::test]
    async fn test_has_similar_hackernews_post() -> Result<()> {
        use crate::models::HackerNewsJobDetail;

        let tmp = temp_db();
        let db = Db::open(tmp.path()).await?;

        let job = |ext: &str, company: &str, role: &str, days_ago: i64| Job {
            id: 0,
            platform: Platform::Hackernews,
            external_id: ext.to_string(),
            title: format!("{} | {}", company, role),
            description: None,
            url: format!("https://news.ycombinator.com/item?id={}", ext),
            budget: None,
            tags: vec![],
            raw: Data::Hackernews {
                detail: HackerNewsJobDetail {
                    author: "whoishiring".to_string(),
                    author_threads_url: "https://news.ycombinator.com/threads?id=whoishiring"
                        .to_string(),
                    company: Some(company.to_string()),
                    role: Some(role.to_string()),
                    location: None,
                },
            },
            company: None,
            created_at: chrono::Utc::now() - chrono::Duration::days(days_ago),
            updated_at: chrono::Utc::now(),
            liked: None,
            note: None,
            applied_at: None,
            remote: true,
        };

        db.upsert_job(&job("hn-1", "Acme", "Senior Rust Engineer", 70))
            .await?;

        let since = chrono::Utc::now() - chrono::Duration::days(90);
        let similar = db
            .has_similar_hackernews_post("Acme", "Senior Rust Engineer", since)
            .await?;
        assert!(similar, "should find similar post after cutoff");

        let different_role = db
            .has_similar_hackernews_post("Acme", "Frontend Engineer", since)
            .await?;
        assert!(!different_role, "different role is not a duplicate");

        let outside_cutoff = chrono::Utc::now() - chrono::Duration::days(30);
        let outside = db
            .has_similar_hackernews_post("Acme", "Senior Rust Engineer", outside_cutoff)
            .await?;
        assert!(!outside, "post before cutoff is not a duplicate");

        Ok(())
    }

    #[tokio::test]
    async fn test_list_jobs_filtered_applied() -> Result<()> {
        let tmp = temp_db();
        let db = Db::open(tmp.path()).await?;

        let id1 = db
            .upsert_job(&test_job(Platform::Upwork, "u1", "Applied"))
            .await?;
        db.upsert_job(&test_job(Platform::Upwork, "u2", "Not applied"))
            .await?;
        db.set_applied(id1, None, chrono::Utc::now()).await?;

        let applied = db
            .list_jobs_filtered(
                &JobFilter {
                    platform: Some(Platform::Upwork),
                    applied: Some(true),
                    ..Default::default()
                },
                Sort::Created,
                10,
                0,
            )
            .await?;
        assert_eq!(applied.items.len(), 1);
        assert_eq!(applied.items[0].title, "Applied");

        let not_applied = db
            .list_jobs_filtered(
                &JobFilter {
                    platform: Some(Platform::Upwork),
                    applied: Some(false),
                    ..Default::default()
                },
                Sort::Created,
                10,
                0,
            )
            .await?;
        assert_eq!(not_applied.items.len(), 1);
        assert_eq!(not_applied.items[0].title, "Not applied");

        let all = db
            .list_jobs_filtered(
                &JobFilter {
                    platform: Some(Platform::Upwork),
                    ..Default::default()
                },
                Sort::Created,
                10,
                0,
            )
            .await?;
        assert_eq!(all.items.len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_list_jobs_filtered_remote() -> Result<()> {
        let tmp = temp_db();
        let db = Db::open(tmp.path()).await?;

        let mut remote_job = test_job(Platform::Upwork, "remote-1", "Remote job");
        remote_job.remote = true;
        let mut onsite_job = test_job(Platform::Upwork, "onsite-1", "Onsite job");
        onsite_job.remote = false;

        db.upsert_job(&remote_job).await?;
        db.upsert_job(&onsite_job).await?;

        let remote = db
            .list_jobs_filtered(
                &JobFilter {
                    platform: Some(Platform::Upwork),
                    remote: Some(true),
                    ..Default::default()
                },
                Sort::Created,
                10,
                0,
            )
            .await?;
        assert_eq!(remote.items.len(), 1);
        assert!(remote.items[0].remote);

        let onsite = db
            .list_jobs_filtered(
                &JobFilter {
                    platform: Some(Platform::Upwork),
                    remote: Some(false),
                    ..Default::default()
                },
                Sort::Created,
                10,
                0,
            )
            .await?;
        assert_eq!(onsite.items.len(), 1);
        assert!(!onsite.items[0].remote);

        let all = db
            .list_jobs_filtered(
                &JobFilter {
                    platform: Some(Platform::Upwork),
                    ..Default::default()
                },
                Sort::Created,
                10,
                0,
            )
            .await?;
        assert_eq!(all.items.len(), 2);

        Ok(())
    }
}
