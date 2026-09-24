//! LLM job-fit classification via the patterns SystemOne (Jev) client.

use anyhow::{Result, ensure};
use patterns::SystemOne;
use patterns::systemone::{Noul, SharedSystemOne};
use serde::Deserialize;

use crate::db::Db;
use crate::models::{Job, JobFilter, Rating};

/// Probability at or above which a job is rated [`Rating::Liked`].
pub const FIT_THRESHOLD: f32 = 0.6;

#[derive(Debug, Clone, Deserialize, SystemOne)]
#[systemone(template = "job_fit.md", healthcheck = "job_fit_healthcheck.md")]
pub struct JobFit {
    #[noul("Given the CV, is this job relevant and would the candidate be a good fit?")]
    pub is_good_fit: Noul,
}

impl JobFit {
    fn verify(&self) -> Result<()> {
        ensure!(
            (0.0..=1.0).contains(&self.is_good_fit.noul),
            "noul out of range: {}",
            self.is_good_fit.noul
        );
        Ok(())
    }
}

/// Map a fit probability to a [`Rating`].
#[must_use]
pub fn rating_for(probability: f32) -> Rating {
    if probability >= FIT_THRESHOLD {
        Rating::Liked
    } else {
        Rating::Disliked
    }
}

/// Default to neutral-only classification unless `force`.
#[must_use]
pub fn with_default_neutral(mut filter: JobFilter, force: bool) -> JobFilter {
    if !force {
        filter.rating = Some(Rating::Neutral);
    }
    filter
}

/// Classify `jobs` against `cv`, setting liked/disliked ratings on `db`.
pub async fn classify_jobs(jev: &SharedSystemOne, cv: &str, db: &Db, jobs: &[Job]) -> Result<()> {
    jev.verify::<JobFit>().await?;

    let mut liked: Vec<i64> = Vec::new();
    let mut liked_jobs: Vec<&Job> = Vec::new();
    let mut disliked: Vec<i64> = Vec::new();
    for job in jobs {
        let text = job.advert_text();
        let answer = jev.evaluate_text::<JobFit>(&text, cv).await?;
        let probability = answer.is_good_fit.noul;
        let rating = rating_for(probability);
        eprintln!(
            "[{}] {} -> {rating:?} ({probability:.2})",
            job.id, job.title
        );
        match rating {
            Rating::Liked => {
                liked.push(job.id);
                liked_jobs.push(job);
            }
            Rating::Disliked => disliked.push(job.id),
            Rating::Neutral => unreachable!("rating_for never returns Neutral"),
        }
    }

    if !liked.is_empty() {
        db.set_rating(&liked, Rating::Liked).await?;
    }
    if !disliked.is_empty() {
        db.set_rating(&disliked, Rating::Disliked).await?;
    }
    println!(
        "Classified {} job{}: {} liked, {} disliked",
        jobs.len(),
        if jobs.len() == 1 { "" } else { "s" },
        liked.len(),
        disliked.len()
    );
    if !liked_jobs.is_empty() {
        println!("Liked this run:");
        for job in liked_jobs {
            println!("  [{}] {} | {}", job.id, job.title, job.platform);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Platform;

    #[test]
    fn rating_for_threshold_boundary() {
        assert_eq!(rating_for(0.6), Rating::Liked);
        assert_eq!(rating_for(0.99), Rating::Liked);
        assert_eq!(rating_for(0.599), Rating::Disliked);
        assert_eq!(rating_for(0.0), Rating::Disliked);
    }

    #[test]
    fn default_neutral_overrides_unless_force() {
        let filter = JobFilter {
            platform: Some(Platform::Upwork),
            rating: Some(Rating::Liked),
            ..JobFilter::default()
        };
        assert_eq!(
            with_default_neutral(filter.clone(), false).rating,
            Some(Rating::Neutral)
        );
        assert_eq!(
            with_default_neutral(filter, true).rating,
            Some(Rating::Liked)
        );
    }

    #[test]
    fn default_neutral_from_none() {
        assert_eq!(
            with_default_neutral(JobFilter::default(), false).rating,
            Some(Rating::Neutral)
        );
    }

    #[tokio::test]
    #[ignore = "live: requires TYPESAFE_API_KEY"]
    async fn live_verify_job_fit() {
        let jev = crate::config::shared_systemone(&crate::config::SystemOneConfig::default())
            .expect("systemone client");
        jev.verify::<JobFit>().await.expect("job-fit healthcheck");
    }

    #[tokio::test]
    #[ignore = "live: requires TYPESAFE_API_KEY"]
    async fn live_classify_end_to_end() -> Result<()> {
        use crate::models::{Data, HackerNewsJobDetail, NewJob, Platform};

        let tmp = tempfile::NamedTempFile::new()?;
        let db = Db::open(tmp.path()).await?;
        let job = NewJob {
            platform: Platform::Hackernews,
            external_id: "live-classify-1".to_owned(),
            title: "Senior Rust Engineer".to_owned(),
            url: "https://example.com/live-classify-1".to_owned(),
            budget: Some("EUR 90k-110k".to_owned()),
            tags: vec!["rust".to_owned()],
            company: Some("Acme".to_owned()),
            created_at: chrono::Utc::now(),
            remote: true,
            raw: Data::Hackernews {
                detail: HackerNewsJobDetail {
                    description: "Fully remote within Europe. Senior Rust backend engineer, Tokio, distributed systems."
                        .to_owned(),
                    ..HackerNewsJobDetail::default()
                },
            },
        };
        let id = db.upsert_job(&job).await?.id();
        let jobs = db.get_jobs(&[id]).await?;

        let cv =
            "Senior Rust backend engineer, 10 years, distributed systems, based in Europe, remote.";
        let jev = crate::config::shared_systemone(&crate::config::SystemOneConfig::default())?;
        classify_jobs(&jev, cv, &db, &jobs).await?;

        let stored = db.get_job(id).await?.expect("job persisted").rating;
        assert_ne!(stored, Rating::Neutral, "classify must set a rating");
        Ok(())
    }
}
