use crate::browser::BrowserExt;
use crate::models::{Job, JobStatus, Platform, Reaction};
use crate::platforms::PlatformClient;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chromiumoxide::browser::Browser;
use serde_json::Value;

#[async_trait]
impl PlatformClient for NoFluffJobsScraper {
    fn name(&self) -> &'static str {
        "nofluffjobs"
    }

    async fn fetch_with_browser(&self, browser: &Browser, query: &str) -> Result<Vec<Job>> {
        let search_url = format!(
            "https://nofluffjobs.com/pl/jobs?criteria=keyword%3D{}",
            urlencoding::encode(query)
        );
        let page = browser.new_tab(&search_url).await?;

        let mut found = false;
        for _ in 0..60 {
            if page.find_element("a.posting-list-item").await.is_ok() {
                found = true;
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        if !found {
            anyhow::bail!("NoFluffJobs job cards did not appear within 30s");
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let raw_jobs: Vec<serde_json::Value> = page
            .evaluate(
                r#"
                Array.from(document.querySelectorAll("a.posting-list-item")).map(el => {
                    const titleEl = el.querySelector('h3');
                    const tagsEls = el.querySelectorAll('.posting-tag');
                    const salaryEl = el.querySelector('[class*="salary"]');
                    const timeEl = el.querySelector('time');
                    const url = el.href;
                    const idMatch = url?.match(/\/job\/([^\/?#]+)/);
                    const id = idMatch ? idMatch[1] : '';
                    return {
                        external_id: id,
                        title: titleEl?.textContent?.trim() || "",
                        description: null,
                        url: url || "",
                        budget: salaryEl?.textContent?.trim() || null,
                        posted_at_text: timeEl?.textContent?.trim() || null,
                        tags: Array.from(tagsEls).map(s => s.textContent.trim()).filter(Boolean)
                    };
                })
                "#,
            )
            .await?
            .into_value()?;

        let jobs: Vec<Job> = raw_jobs
            .iter()
            .filter_map(|v| raw_to_job(v).ok())
            .filter(|j| !j.external_id.is_empty())
            .collect();

        page.close().await.ok();
        Ok(jobs)
    }

    async fn react(&self, _job: &Job, _action: Reaction) -> Result<()> {
        anyhow::bail!("NoFluffJobs react not yet implemented")
    }
}

fn raw_to_job(v: &Value) -> Result<Job> {
    let external_id = v["external_id"]
        .as_str()
        .ok_or_else(|| anyhow!("missing external_id"))?
        .to_string();

    let posted_at = v["posted_at_text"].as_str().and_then(parse_nofluff_time);

    Ok(Job {
        id: None,
        platform: Platform::NoFluffJobs,
        external_id,
        title: v["title"].as_str().unwrap_or("").to_string(),
        description: v["description"].as_str().map(|s| s.to_string()),
        url: v["url"].as_str().unwrap_or("").to_string(),
        posted_at,
        budget: v["budget"].as_str().map(|s| s.to_string()),
        tags: v["tags"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        raw: v.clone(),
        status: JobStatus::New,
        created_at: None,
        updated_at: None,
    })
}

fn parse_nofluff_time(text: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let now = chrono::Utc::now();
    let text = text.to_lowercase();
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    let n: i64 = parts[0].parse().ok()?;
    match parts[1] {
        "minute" | "minutes" => Some(now - chrono::Duration::minutes(n)),
        "hour" | "hours" => Some(now - chrono::Duration::hours(n)),
        "day" | "days" => Some(now - chrono::Duration::days(n)),
        "week" | "weeks" => Some(now - chrono::Duration::days(n * 7)),
        "month" | "months" => Some(now - chrono::Duration::days(n * 30)),
        "yesterday" => Some(now - chrono::Duration::days(1)),
        _ => None,
    }
}

pub struct NoFluffJobsScraper;

impl Default for NoFluffJobsScraper {
    fn default() -> Self {
        Self::new()
    }
}

impl NoFluffJobsScraper {
    pub fn new() -> Self {
        Self
    }
}
