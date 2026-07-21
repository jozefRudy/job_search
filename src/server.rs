use crate::cli::VERSION;
use crate::db::Db;
use crate::embed::{DEFAULT_EMBEDDING_MODEL, Embedder};
use crate::embeddings_store::EmbeddingsStore;
use crate::models::{
    ApplyRequest, Data, HackerNewsJobDetail, Job, JobFilter, JobListResponse, ListQuery,
    NoFluffJobDetail, Platform, RateRequest, Rating, Sort, UpworkJobDetail,
};
use anyhow::{Error, Result};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::Utc;
use include_dir::{Dir, include_dir};
use std::sync::Arc;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

#[derive(OpenApi)]
#[openapi(components(schemas(
    Job,
    JobListResponse,
    RateRequest,
    ApplyRequest,
    Data,
    UpworkJobDetail,
    NoFluffJobDetail,
    HackerNewsJobDetail,
    Platform,
    Rating,
    Sort
)))]
struct ApiDoc;

static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/frontend/dist");

pub struct AppState {
    pub db: Db,
    pub embeddings: Arc<EmbeddingsStore>,
}

pub fn app(db: Db, embeddings: Arc<EmbeddingsStore>) -> Router {
    let state = Arc::new(AppState { db, embeddings });

    let (api_router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(list_jobs))
        .routes(routes!(get_job, delete_job))
        .routes(routes!(rate_job))
        .routes(routes!(apply_job))
        .with_state(state.clone())
        .split_for_parts();

    api_router
        .route("/api/openapi.json", get(move || async move { Json(api) }))
        .route("/health", get(|| async { StatusCode::OK }))
        .fallback(serve_static)
}

fn internal_error(err: &Error) -> StatusCode {
    eprintln!("internal server error: {err:#}");
    StatusCode::INTERNAL_SERVER_ERROR
}

async fn serve_static(uri: axum::http::Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    let served_path = if STATIC_DIR.get_file(path).is_some() {
        path
    } else {
        "index.html"
    };
    let Some(file) = STATIC_DIR.get_file(served_path) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let content_type = mime_guess::from_path(served_path)
        .first_raw()
        .unwrap_or("application/octet-stream");
    ([(header::CONTENT_TYPE, content_type)], file.contents()).into_response()
}

#[utoipa::path(
    get,
    path = "/api/jobs",
    params(ListQuery),
    responses(
        (status = 200, description = "List of jobs", body = JobListResponse),
        (status = 500, description = "Internal server error")
    )
)]
async fn list_jobs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<JobListResponse>, StatusCode> {
    let page = query.page.max(1);
    let page_size = query.page_size.clamp(1, 100);
    let offset = i64::try_from((page - 1) * page_size).unwrap_or(i64::MAX);
    let limit = i64::try_from(page_size).unwrap_or(i64::MAX);

    let filter = JobFilter {
        platform: query.platform,
        rating: query.rating,
        applied: query.applied,
        remote: query.remote,
    };

    let search = query.search.as_deref().filter(|s| !s.is_empty());
    let sort_by = if search.is_some() {
        Sort::Relevance
    } else if query.sort_by == Sort::Relevance {
        Sort::Created
    } else {
        query.sort_by
    };

    if let Some(query_text) = search {
        let candidate_ids = state
            .db
            .filter_job_ids(&filter)
            .await
            .map_err(|err| internal_error(&err))?;

        let vectorized_count = state
            .embeddings
            .count_vectorized_candidates(&candidate_ids)
            .await
            .map_err(|err| internal_error(&err))?;
        if vectorized_count == 0 {
            return Ok(Json(JobListResponse {
                jobs: vec![],
                total: 0,
            }));
        }

        let query_embedding = state
            .embeddings
            .embedder()
            .embed_query(query_text)
            .await
            .map_err(|err| internal_error(&err))?;
        let top_n = usize::try_from(limit + offset).unwrap_or(usize::MAX);
        let ranked = state
            .embeddings
            .search(&query_embedding, &candidate_ids, top_n, 0)
            .await
            .map_err(|err| internal_error(&err))?;
        let ids: Vec<i64> = ranked
            .into_iter()
            .map(|(id, _)| id)
            .skip(usize::try_from(offset).unwrap_or(usize::MAX))
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
            .collect();
        let jobs = state
            .db
            .get_jobs(&ids)
            .await
            .map_err(|err| internal_error(&err))?;
        return Ok(Json(JobListResponse {
            jobs,
            total: vectorized_count,
        }));
    }

    let paginated = state
        .db
        .list_jobs_filtered(&filter, sort_by, limit, offset)
        .await
        .map_err(|err| internal_error(&err))?;

    Ok(Json(JobListResponse {
        jobs: paginated.items,
        total: usize::try_from(paginated.total).unwrap_or(usize::MAX),
    }))
}

#[utoipa::path(
    get,
    path = "/api/jobs/{id}",
    params(("id" = i64, Path, description = "Job ID")),
    responses(
        (status = 200, description = "Job details", body = Job),
        (status = 404, description = "Job not found"),
        (status = 500, description = "Internal server error")
    )
)]
async fn get_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Job>, StatusCode> {
    let job = state
        .db
        .get_job(id)
        .await
        .map_err(|err| internal_error(&err))?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(job))
}

#[utoipa::path(
    post,
    path = "/api/jobs/{id}/rate",
    params(("id" = i64, Path, description = "Job ID")),
    request_body = RateRequest,
    responses(
        (status = 204, description = "Rating updated"),
        (status = 500, description = "Internal server error")
    )
)]
async fn rate_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<RateRequest>,
) -> Result<StatusCode, StatusCode> {
    state
        .db
        .set_rating(&[id], body.rating)
        .await
        .map_err(|err| internal_error(&err))?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete,
    path = "/api/jobs/{id}",
    params(("id" = i64, Path, description = "Job ID")),
    responses(
        (status = 204, description = "Job deleted"),
        (status = 404, description = "Job not found"),
        (status = 500, description = "Internal server error")
    )
)]
async fn delete_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    let deleted = state
        .db
        .delete_jobs(&[id])
        .await
        .map_err(|err| internal_error(&err))?;
    if deleted == 1 {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

#[utoipa::path(
    post,
    path = "/api/jobs/{id}/apply",
    params(("id" = i64, Path, description = "Job ID")),
    request_body = ApplyRequest,
    responses(
        (status = 204, description = "Application state updated"),
        (status = 500, description = "Internal server error")
    )
)]
async fn apply_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<ApplyRequest>,
) -> Result<StatusCode, StatusCode> {
    if body.applied {
        state
            .db
            .set_applied(id, Some(""), Utc::now())
            .await
            .map_err(|err| internal_error(&err))?;
    } else {
        state
            .db
            .unset_applied(id)
            .await
            .map_err(|err| internal_error(&err))?;
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn shutdown_signal() {
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = term.recv() => {}
    }
}

pub async fn serve(db: Db, db_path: &std::path::Path, port: u16) -> Result<()> {
    let cache_dir = db_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let embedder = Embedder::load(cache_dir).await?;
    let embeddings = Arc::new(
        EmbeddingsStore::open(db_path, DEFAULT_EMBEDDING_MODEL, db.clone(), embedder).await?,
    );
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    eprintln!("jobsearch ({VERSION}) listening on http://0.0.0.0:{port}");
    axum::serve(listener, app(db, embeddings))
        .with_graceful_shutdown(async {
            shutdown_signal().await;
            eprintln!("jobsearch ({VERSION}) shutting down");
        })
        .await?;
    Ok(())
}
