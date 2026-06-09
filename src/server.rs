use crate::db::Db;
use crate::models::{Data, Job, Platform, Rating};
use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use include_dir::{Dir, include_dir};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/frontend/dist");

pub struct AppState {
    pub db: Db,
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    platform: Option<String>,
    rating: Option<String>,
    #[serde(default = "default_sort")]
    sort: String,
}

fn default_sort() -> String {
    "created".to_string()
}

#[derive(Debug, Serialize)]
struct JobListResponse {
    jobs: Vec<Job>,
}

#[derive(Debug, Deserialize)]
struct RateBody {
    rating: String,
}

pub fn app(db: Db) -> Router {
    let state = Arc::new(AppState { db });

    Router::new()
        .route("/api/jobs", get(list_jobs))
        .route("/api/jobs/{id}", get(get_job))
        .route("/api/jobs/{id}/rate", post(rate_job))
        .fallback(serve_static)
        .with_state(state)
}

async fn serve_static(uri: axum::http::Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    let file = STATIC_DIR
        .get_file(path)
        .or_else(|| STATIC_DIR.get_file("index.html"));

    if let Some(file) = file {
        let content_type = mime_guess::from_path(path)
            .first_raw()
            .unwrap_or("application/octet-stream");
        ([(header::CONTENT_TYPE, content_type)], file.contents()).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn list_jobs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<JobListResponse>, StatusCode> {
    let platform = query.platform.and_then(|p| match p.as_str() {
        "upwork" => Some(Platform::Upwork),
        "nofluffjobs" => Some(Platform::NoFluffJobs),
        _ => None,
    });

    let liked = query.rating.and_then(|r| match r.as_str() {
        "liked" => Some(Rating::Liked),
        "disliked" => Some(Rating::Disliked),
        "neutral" => Some(Rating::Neutral),
        _ => None,
    });

    let mut jobs = state
        .db
        .list_jobs_filtered(platform, liked, 500)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match query.sort.as_str() {
        "viewed" => {
            jobs.sort_by(|a, b| {
                if let (Data::Upwork { detail: ad }, Data::Upwork { detail: bd }) = (&a.raw, &b.raw)
                {
                    let av = ad.last_viewed;
                    let bv = bd.last_viewed;
                    (bv.is_some(), bv).cmp(&(av.is_some(), av))
                } else {
                    std::cmp::Ordering::Equal
                }
            });
        }
        _ => {
            jobs.sort_by_key(|b| std::cmp::Reverse(b.created_at));
        }
    }

    Ok(Json(JobListResponse { jobs }))
}

async fn get_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Job>, StatusCode> {
    let job = state
        .db
        .get_job(id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(job))
}

async fn rate_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<RateBody>,
) -> Result<StatusCode, StatusCode> {
    match body.rating.as_str() {
        "liked" => state.db.set_liked(&[id], true).await,
        "disliked" => state.db.set_liked(&[id], false).await,
        "neutral" => state.db.set_neutral(&[id]).await,
        _ => return Err(StatusCode::BAD_REQUEST),
    }
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn serve(db: Db, port: u16) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    eprintln!("Server listening on http://0.0.0.0:{}", port);
    axum::serve(listener, app(db)).await?;
    Ok(())
}
