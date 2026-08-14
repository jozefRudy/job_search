use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use arrow::array::{Float32Array, RecordBatch, RecordBatchIterator, as_primitive_array};
use arrow::datatypes::{DataType, Field, Int64Type, Schema, TimeUnit};
use arrow_array::builder::{
    FixedSizeListBuilder, Float32Builder, Int64Builder, StringBuilder, TimestampMicrosecondBuilder,
};
use chrono::Utc;
use futures::TryStreamExt;
use lance::dataset::optimize::{CompactionOptions, compact_files};
use lance::dataset::{
    Dataset, MergeInsertBuilder, WhenMatched, WhenNotMatched, WriteMode, WriteParams,
};
use lance::index::DatasetIndexExt;
use lance::index::vector::VectorIndexParams;
use lance_index::IndexType;
use lance_index::optimize::OptimizeOptions;
use lance_index::scalar::FullTextSearchQuery;
use lance_index::scalar::inverted::InvertedIndexParams;
use lance_index::scalar::inverted::query::{FtsQuery, MatchQuery, Operator};
use lance_index::vector::{hnsw::builder::HnswBuildParams, ivf::IvfBuildParams, pq::PQBuildParams};
use lance_linalg::distance::DistanceType;

use crate::db::Db;
use crate::embed::Embedder;
use crate::models::Job;
use tokio::sync::Mutex;

pub const VECTOR_SEARCH_MAX_RESULTS: usize = 1000;
const RRF_K: f32 = 60.0;

pub struct SearchResult {
    pub total: usize,
    pub capped: bool,
    pub items: Vec<(i64, f32)>,
}

pub struct EmbeddingsStore {
    db: Db,
    embedder: Embedder,
    dim: usize,
    dataset: Mutex<Dataset>,
}

#[must_use]
pub fn embeddings_dir(base: &Path, model_id: &str) -> PathBuf {
    let model_dir = model_id.replace('/', "-");
    base.join("lance").join(format!("embeddings-{model_dir}"))
}

impl EmbeddingsStore {
    pub async fn open(
        sqlite_db_path: &Path,
        model_id: &'static str,
        db: Db,
        embedder: Embedder,
    ) -> Result<Self> {
        let base = sqlite_db_path.parent().unwrap_or_else(|| Path::new("."));
        tokio::fs::create_dir_all(&embeddings_dir(base, model_id)).await?;
        let uri = embeddings_dir(base, model_id).to_string_lossy().to_string();
        let dim = embedder.dim();

        let dataset = if Dataset::open(&uri).await.is_err() {
            let schema = Arc::new(arrow_schema(dim));
            let empty = RecordBatch::new_empty(schema.clone());
            let reader = RecordBatchIterator::new(vec![Ok(empty)], schema.clone());
            let params = WriteParams {
                mode: WriteMode::Create,
                ..Default::default()
            };
            Dataset::write(reader, &uri, Some(params))
                .await
                .context("creating embeddings dataset")?
        } else {
            Dataset::open(&uri).await?
        };

        if dataset.schema().field("search_text").is_none() {
            bail!("embeddings dataset has legacy schema; run `jobsearch embed --force`");
        }

        Ok(Self {
            db,
            embedder,
            dim,
            dataset: Mutex::new(dataset),
        })
    }

    #[must_use]
    pub fn embedder(&self) -> &Embedder {
        &self.embedder
    }

    pub async fn upsert_embedding(&self, job_id: i64, embedding: &[f32], text: &str) -> Result<()> {
        self.upsert_batch(&[job_id], &[embedding.to_vec()], &[text.to_string()])
            .await
    }

    pub async fn upsert_batch(
        &self,
        job_ids: &[i64],
        embeddings: &[Vec<f32>],
        texts: &[String],
    ) -> Result<()> {
        if job_ids.is_empty() {
            return Ok(());
        }

        let batch = embeddings_to_batch(job_ids, embeddings, texts, self.dim)?;
        let schema = batch.schema();
        let reader = RecordBatchIterator::new(vec![Ok(batch)], schema);
        let stream = lance_datafusion::utils::reader_to_stream(Box::new(reader));

        let dataset = self.dataset.lock().await.clone();
        let (new_dataset, _stats) =
            MergeInsertBuilder::try_new(Arc::new(dataset), vec!["job_id".to_string()])?
                .when_matched(WhenMatched::UpdateAll)
                .when_not_matched(WhenNotMatched::InsertAll)
                .try_build()?
                .execute(stream)
                .await
                .context("upserting embeddings")?;

        *self.dataset.lock().await =
            Arc::try_unwrap(new_dataset).unwrap_or_else(|arc| (*arc).clone());

        self.db.mark_vectorized(job_ids).await
    }

    pub async fn get_unvectorized_jobs(&self, limit: i64) -> Result<Vec<Job>> {
        let ids = self.db.get_unvectorized_job_ids(limit).await?;
        self.db.get_jobs(&ids).await
    }

    pub async fn search(
        &self,
        query: &str,
        embedding: &[f32],
        candidate_ids: &[i64],
        limit: usize,
        offset: usize,
    ) -> Result<SearchResult> {
        if candidate_ids.is_empty() || embedding.len() != self.dim {
            return Ok(SearchResult {
                total: 0,
                capped: false,
                items: Vec::new(),
            });
        }

        let mut dataset = self.dataset.lock().await;
        if dataset.is_stale().await? {
            dataset.checkout_latest().await?;
        }

        let id_list = candidate_ids
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        let filter = format!("job_id IN ({id_list})");

        let vector_ranked = self.vector_leg(&dataset, embedding, &filter).await?;
        let fts_ranked = self.fts_leg(&dataset, query, &filter).await?;
        let mut merged = rrf_merge(&vector_ranked, &fts_ranked);
        merged.truncate(VECTOR_SEARCH_MAX_RESULTS);

        let total = merged.len();
        let capped = vector_ranked.len() == VECTOR_SEARCH_MAX_RESULTS
            || fts_ranked.len() == VECTOR_SEARCH_MAX_RESULTS;
        let items = merged.into_iter().skip(offset).take(limit).collect();
        Ok(SearchResult {
            total,
            capped,
            items,
        })
    }

    async fn vector_leg(
        &self,
        dataset: &Dataset,
        embedding: &[f32],
        filter: &str,
    ) -> Result<Vec<i64>> {
        let mut scanner = dataset.scan();
        scanner.filter(filter)?;
        scanner.prefilter(true);
        let query_arr = Float32Array::from(embedding.to_vec());
        scanner.nearest("embedding", &query_arr, VECTOR_SEARCH_MAX_RESULTS)?;
        scanner.distance_metric(DistanceType::Cosine);
        scanner.project(&["job_id"])?;
        collect_ids(scanner).await
    }

    async fn fts_leg(&self, dataset: &Dataset, query: &str, filter: &str) -> Result<Vec<i64>> {
        let mut scanner = dataset.scan();
        scanner.filter(filter)?;
        let fts_query: FtsQuery = MatchQuery::new(query.to_string())
            .with_operator(Operator::Or)
            .with_column(Some("search_text".into()))
            .into();
        scanner.full_text_search(FullTextSearchQuery::new_query(fts_query))?;
        scanner.project(&["job_id"])?;
        scanner.limit(
            Some(i64::try_from(VECTOR_SEARCH_MAX_RESULTS).expect("fits in i64")),
            None,
        )?;
        collect_ids(scanner).await
    }

    pub async fn maintenance(&self) -> Result<()> {
        let mut dataset = self.dataset.lock().await;
        let row_count = dataset.count_rows(None).await.context("counting rows")?;
        let indices = dataset.load_indices().await.context("loading indices")?;

        if row_count >= 256 && !indices.iter().any(|idx| idx.name == "embedding_idx") {
            let vector_params = VectorIndexParams::with_ivf_hnsw_pq_params(
                DistanceType::Cosine,
                IvfBuildParams::default(),
                HnswBuildParams::default(),
                PQBuildParams::default(),
            );
            dataset
                .create_index_builder(&["embedding"], IndexType::IvfHnswPq, &vector_params)
                .name("embedding_idx".into())
                .replace(true)
                .await
                .context("creating vector index")?;
        }

        if row_count > 0 && !indices.iter().any(|idx| idx.name == "search_text_idx") {
            dataset
                .create_index_builder(
                    &["search_text"],
                    IndexType::Inverted,
                    &InvertedIndexParams::default(),
                )
                .name("search_text_idx".into())
                .replace(true)
                .await
                .context("creating inverted index on search_text")?;
        }

        compact_files(&mut dataset, CompactionOptions::default(), None)
            .await
            .context("compacting dataset")?;
        dataset
            .optimize_indices(&OptimizeOptions::default())
            .await
            .context("optimizing indices")?;
        Ok(())
    }

    pub async fn index_unvectorized<F>(
        &self,
        batch_size: usize,
        mut on_progress: F,
    ) -> Result<usize>
    where
        F: FnMut(usize),
    {
        let mut total = 0;
        loop {
            let jobs = self
                .get_unvectorized_jobs(i64::try_from(batch_size).expect("batch size fits in i64"))
                .await?;
            if jobs.is_empty() {
                break;
            }
            let owned_texts: Vec<String> = jobs.iter().map(Job::advert_text).collect();
            let embeddings = self.embedder.embed_batch_documents(&owned_texts).await?;
            let ids: Vec<i64> = jobs.iter().map(|job| job.id).collect();
            self.upsert_batch(&ids, &embeddings, &owned_texts).await?;
            total += ids.len();
            on_progress(total);
        }
        self.maintenance().await?;
        Ok(total)
    }

    #[cfg(test)]
    async fn list_vectorized_ids(&self) -> Result<Vec<i64>> {
        let mut dataset = self.dataset.lock().await;
        if dataset.is_stale().await? {
            dataset.checkout_latest().await?;
        }
        let mut scanner = dataset.scan();
        scanner.project(&["job_id"])?;
        let batches: Vec<RecordBatch> = scanner.try_into_stream().await?.try_collect().await?;
        let mut ids = Vec::new();
        for batch in &batches {
            let arr = as_primitive_array::<Int64Type>(
                batch.column_by_name("job_id").context("job_id column")?,
            );
            for i in 0..batch.num_rows() {
                ids.push(arr.value(i));
            }
        }
        Ok(ids)
    }
}

fn arrow_schema(dim: usize) -> Schema {
    Schema::new(vec![
        Field::new("job_id", DataType::Int64, false),
        Field::new(
            "embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                i32::try_from(dim).expect("embedding dimension fits in i32"),
            ),
            false,
        ),
        Field::new("search_text", DataType::Utf8, false),
        Field::new(
            "created_at",
            DataType::Timestamp(TimeUnit::Microsecond, None),
            false,
        ),
    ])
}

fn embeddings_to_batch(
    job_ids: &[i64],
    embeddings: &[Vec<f32>],
    texts: &[String],
    dim: usize,
) -> Result<RecordBatch> {
    if job_ids.len() != embeddings.len() || job_ids.len() != texts.len() {
        bail!("job_ids, embeddings and texts must have the same length");
    }

    let mut id_builder = Int64Builder::new();
    let mut emb_builder = FixedSizeListBuilder::new(
        Float32Builder::new(),
        i32::try_from(dim).expect("embedding dimension fits in i32"),
    );
    let mut text_builder = StringBuilder::new();
    let mut ts_builder = TimestampMicrosecondBuilder::new();
    let now = Utc::now().timestamp_micros();

    for ((id, emb), text) in job_ids.iter().zip(embeddings.iter()).zip(texts.iter()) {
        if emb.len() != dim {
            bail!("expected embedding dimension {dim}, got {}", emb.len());
        }
        id_builder.append_value(*id);
        emb_builder.values().append_slice(emb);
        emb_builder.append(true);
        text_builder.append_value(text);
        ts_builder.append_value(now);
    }

    let schema = Arc::new(arrow_schema(dim));
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id_builder.finish()),
            Arc::new(emb_builder.finish()),
            Arc::new(text_builder.finish()),
            Arc::new(ts_builder.finish()),
        ],
    )?;
    Ok(batch)
}

async fn collect_ids(scanner: lance::dataset::scanner::Scanner) -> Result<Vec<i64>> {
    let batches: Vec<RecordBatch> = scanner.try_into_stream().await?.try_collect().await?;
    let mut ids = Vec::new();
    for batch in &batches {
        let arr = as_primitive_array::<Int64Type>(
            batch.column_by_name("job_id").context("job_id column")?,
        );
        for i in 0..batch.num_rows() {
            ids.push(arr.value(i));
        }
    }
    Ok(ids)
}

fn rrf_merge(vector_ranked: &[i64], fts_ranked: &[i64]) -> Vec<(i64, f32)> {
    let mut scores: HashMap<i64, f32> = HashMap::new();
    for (rank, id) in vector_ranked.iter().enumerate() {
        *scores.entry(*id).or_default() += 1.0 / (RRF_K + rank as f32 + 1.0);
    }
    for (rank, id) in fts_ranked.iter().enumerate() {
        *scores.entry(*id).or_default() += 1.0 / (RRF_K + rank as f32 + 1.0);
    }
    let mut merged: Vec<(i64, f32)> = scores.into_iter().collect();
    merged.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::DEFAULT_EMBEDDING_MODEL;

    #[test]
    fn rrf_merge_disjoint_max_legs_truncated_to_max() {
        let max = i64::try_from(VECTOR_SEARCH_MAX_RESULTS).expect("fits in i64");
        let vector_leg: Vec<i64> = (0..max).collect();
        let fts_leg: Vec<i64> = (max..2 * max).collect();
        let mut merged = rrf_merge(&vector_leg, &fts_leg);
        assert_eq!(merged.len(), 2 * VECTOR_SEARCH_MAX_RESULTS);
        merged.truncate(VECTOR_SEARCH_MAX_RESULTS);
        assert_eq!(merged.len(), VECTOR_SEARCH_MAX_RESULTS);
    }

    #[test]
    fn rrf_merge_unions_both_legs() {
        let merged = rrf_merge(&[1, 2], &[3]);
        let ids: Vec<i64> = merged.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&1) && ids.contains(&2) && ids.contains(&3));
    }

    #[test]
    fn rrf_merge_boosts_ids_present_in_both_legs() {
        // id 2 is rank 1 in vector leg and rank 0 in fts leg;
        // id 1 is rank 0 in vector leg only -> 2 must outrank 1
        let merged = rrf_merge(&[1, 2], &[2]);
        assert_eq!(merged[0].0, 2);
        assert!(merged[0].1 > merged[1].1);
    }

    #[test]
    fn rrf_merge_respects_rank_order_within_leg() {
        let merged = rrf_merge(&[1, 2, 3], &[]);
        let ids: Vec<i64> = merged.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn rrf_merge_breaks_ties_by_id() {
        let merged = rrf_merge(&[2, 1], &[1, 2]);
        assert_eq!(merged[0].0, 1);
        assert_eq!(merged[1].0, 2);
        assert!((merged[0].1 - merged[1].1).abs() < f32::EPSILON);
    }

    #[test]
    fn rrf_merge_empty_legs() {
        assert!(rrf_merge(&[], &[]).is_empty());
        let merged = rrf_merge(&[], &[7]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].0, 7);
    }

    #[test]
    fn rrf_merge_scores_match_formula() {
        let merged = rrf_merge(&[5], &[5]);
        let expected = 1.0 / (RRF_K + 1.0) + 1.0 / (RRF_K + 1.0);
        assert!((merged[0].1 - expected).abs() < 1e-7);
    }

    const TEST_DIM: usize = 768;
    use crate::models::{
        Data, EfinancialcareersJobDetail, LinkedInJobDetail, NewJob, NoFluffJobDetail, Platform,
        UpworkJobDetail,
    };

    fn test_job(platform: Platform, external_id: &str, title: &str) -> NewJob {
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
            Platform::LinkedIn => Data::LinkedIn {
                detail: LinkedInJobDetail::default(),
            },
            Platform::Reddit => Data::Reddit {
                detail: crate::models::RedditJobDetail::default(),
            },
            Platform::Workatastartup => Data::Workatastartup {
                detail: crate::models::WorkAtStartupJobDetail::default(),
            },
            Platform::Wellfound => Data::Wellfound {
                detail: crate::models::WellfoundJobDetail::default(),
            },
        };
        NewJob {
            platform,
            external_id: external_id.to_string(),
            title: title.to_string(),
            url: format!("https://example.com/{external_id}"),
            budget: None,
            tags: vec![],
            raw,
            company: None,
            created_at: chrono::Utc::now(),
            remote: true,
        }
    }

    async fn test_store() -> (tempfile::TempDir, Db, EmbeddingsStore) {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");
        let db = Db::open(&db_path).await.unwrap();
        let embedder = Embedder::fake(TEST_DIM);
        let store = EmbeddingsStore::open(&db_path, DEFAULT_EMBEDDING_MODEL, db.clone(), embedder)
            .await
            .unwrap();
        (tmp, db, store)
    }

    #[tokio::test]
    async fn open_creates_dataset() {
        let (_tmp, _db, store) = test_store().await;
        let uri = store.dataset.lock().await.uri().to_string();
        let dataset = Dataset::open(&uri).await.unwrap();
        assert_eq!(dataset.schema().fields.len(), 4);
    }

    #[tokio::test]
    async fn open_uses_lance_subdirectory() {
        let (tmp, _db, store) = test_store().await;
        let expected = tmp
            .path()
            .join("lance")
            .join("embeddings-nomic-ai-nomic-embed-text-v1.5");
        assert_eq!(store.dataset.lock().await.uri(), expected.to_string_lossy());
    }

    #[tokio::test]
    async fn index_unvectorized_embeddings() {
        let (_tmp, db, store) = test_store().await;
        let _ = db
            .upsert_job(&test_job(Platform::Upwork, "u1", "Rust backend developer"))
            .await
            .unwrap()
            .id();
        let _ = db
            .upsert_job(&test_job(
                Platform::NoFluffJobs,
                "n1",
                "Senior Python engineer",
            ))
            .await
            .unwrap()
            .id();

        let indexed = store
            .index_unvectorized(16, |_total| {
                // no-op progress callback in test
            })
            .await
            .unwrap();
        assert_eq!(indexed, 2);

        let vectorized = store.list_vectorized_ids().await.unwrap();
        assert_eq!(vectorized.len(), 2);
    }

    #[tokio::test]
    async fn search_returns_ranked_job_ids() {
        let (_tmp, db, store) = test_store().await;
        let id1 = db
            .upsert_job(&test_job(Platform::Upwork, "u1", "Rust backend developer"))
            .await
            .unwrap()
            .id();
        let id2 = db
            .upsert_job(&test_job(
                Platform::NoFluffJobs,
                "n2",
                "Python data scientist",
            ))
            .await
            .unwrap()
            .id();

        let mut query = vec![0.0f32; TEST_DIM];
        query[0] = 1.0;
        let mut emb1 = vec![0.0f32; TEST_DIM];
        emb1[0] = 1.0;
        let mut emb2 = vec![0.0f32; TEST_DIM];
        emb2[1] = 1.0;
        store
            .upsert_batch(
                &[id1, id2],
                &[emb1, emb2],
                &["rust backend".to_string(), "python data".to_string()],
            )
            .await
            .unwrap();

        let result = store
            .search("rust", &query, &[id1, id2], 10, 0)
            .await
            .unwrap();
        let ranked = result.items;
        assert_eq!(result.total, 2);
        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].0, id1);
    }

    #[tokio::test]
    async fn search_respects_candidate_filter() {
        let (_tmp, db, store) = test_store().await;
        let id1 = db
            .upsert_job(&test_job(Platform::Upwork, "u1", "Rust backend developer"))
            .await
            .unwrap()
            .id();
        let id2 = db
            .upsert_job(&test_job(
                Platform::NoFluffJobs,
                "n2",
                "Python data scientist",
            ))
            .await
            .unwrap()
            .id();

        let mut query = vec![0.0f32; TEST_DIM];
        query[0] = 1.0;
        let mut emb1 = vec![0.0f32; TEST_DIM];
        emb1[0] = 1.0;
        let mut emb2 = vec![0.0f32; TEST_DIM];
        emb2[1] = 1.0;
        store
            .upsert_batch(
                &[id1, id2],
                &[emb1, emb2],
                &["rust backend".to_string(), "python data".to_string()],
            )
            .await
            .unwrap();

        let result = store.search("rust", &query, &[id2], 10, 0).await.unwrap();
        let ranked = result.items;
        assert_eq!(result.total, 1);
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].0, id2);
    }

    #[tokio::test]
    async fn search_total_is_not_limited_by_page_size() {
        let (_tmp, db, store) = test_store().await;
        let mut ids = Vec::new();
        for i in 0..30 {
            let mut job = test_job(Platform::Upwork, &format!("u{i}"), &format!("Job {i}"));
            if let Data::Upwork { ref mut detail } = job.raw {
                detail.description = "same description".to_string();
            }
            let id = db.upsert_job(&job).await.unwrap().id();
            ids.push(id);
        }

        let embedding = vec![1.0f32; TEST_DIM];
        let embeddings: Vec<Vec<f32>> = (0..30).map(|_| embedding.clone()).collect();
        let texts: Vec<String> = (0..30).map(|_| "same description".to_string()).collect();
        store.upsert_batch(&ids, &embeddings, &texts).await.unwrap();

        let query = vec![1.0f32; TEST_DIM];
        let result = store.search("zzz", &query, &ids, 10, 0).await.unwrap();
        assert_eq!(result.items.len(), 10);
        assert_eq!(
            result.total, 30,
            "total should be the full candidate count, not the page size"
        );
    }

    #[tokio::test]
    async fn multiple_consecutive_searches_after_write() {
        let (_tmp, db, store) = test_store().await;
        let id1 = db
            .upsert_job(&test_job(Platform::Upwork, "u1", "Rust backend developer"))
            .await
            .unwrap()
            .id();

        let mut emb1 = vec![0.0f32; TEST_DIM];
        emb1[0] = 1.0;
        store
            .upsert_batch(&[id1], &[emb1], &["rust backend".to_string()])
            .await
            .unwrap();

        let mut query = vec![0.0f32; TEST_DIM];
        query[0] = 1.0;
        for i in 0..30 {
            let result = store.search("zzz", &query, &[id1], 10, 0).await.unwrap();
            let ranked = result.items;
            assert_eq!(result.total, 1, "search {i} should return one result");
            assert_eq!(ranked.len(), 1, "search {i} should return one result");
            assert_eq!(ranked[0].0, id1);
        }
    }

    #[tokio::test]
    async fn search_ranks_by_vector_similarity() {
        let (_tmp, db, store) = test_store().await;
        let id1 = db
            .upsert_job(&test_job(Platform::Upwork, "u1", "Rust backend developer"))
            .await
            .unwrap()
            .id();
        let id2 = db
            .upsert_job(&test_job(
                Platform::NoFluffJobs,
                "n2",
                "Python data scientist",
            ))
            .await
            .unwrap()
            .id();

        let mut query = vec![0.0f32; TEST_DIM];
        query[0] = 1.0;
        let mut emb_same = vec![0.0f32; TEST_DIM];
        emb_same[0] = 2.0;
        let mut emb_orth = vec![0.0f32; TEST_DIM];
        emb_orth[1] = 1.0;
        store
            .upsert_batch(
                &[id1, id2],
                &[emb_same, emb_orth],
                &["rust backend".to_string(), "python data".to_string()],
            )
            .await
            .unwrap();

        // query term matches nothing in search_text: FTS leg empty, pure vector ranking
        let result = store
            .search("zzz", &query, &[id1, id2], 10, 0)
            .await
            .unwrap();
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.items[0].0, id1);
        assert!(
            result.items[0].1 > result.items[1].1,
            "same-direction vector should outrank orthogonal one"
        );
    }

    #[tokio::test]
    async fn hybrid_search_surfaces_exact_term_matches_missed_by_vector() {
        let (_tmp, db, store) = test_store().await;
        let id1 = db
            .upsert_job(&test_job(Platform::Upwork, "u1", "Rust backend developer"))
            .await
            .unwrap()
            .id();
        let id2 = db
            .upsert_job(&test_job(
                Platform::NoFluffJobs,
                "n2",
                "Python data scientist",
            ))
            .await
            .unwrap()
            .id();

        // vector search ranks id1 far above id2
        let mut query = vec![0.0f32; TEST_DIM];
        query[0] = 1.0;
        let mut emb1 = vec![0.0f32; TEST_DIM];
        emb1[0] = 1.0;
        let mut emb2 = vec![0.0f32; TEST_DIM];
        emb2[1] = 1.0;
        store
            .upsert_batch(
                &[id1, id2],
                &[emb1, emb2],
                &[
                    "generic description".to_string(),
                    "deploy to kubernetes clusters".to_string(),
                ],
            )
            .await
            .unwrap();

        let result = store
            .search("kubernetes", &query, &[id1, id2], 10, 0)
            .await
            .unwrap();
        assert_eq!(
            result.items[0].0, id2,
            "exact term match should be boosted to top via FTS leg"
        );
    }

    #[tokio::test]
    async fn upsert_overwrites_existing_embedding() {
        let (_tmp, db, store) = test_store().await;
        let id1 = db
            .upsert_job(&test_job(Platform::Upwork, "u1", "Rust backend developer"))
            .await
            .unwrap()
            .id();
        let id2 = db
            .upsert_job(&test_job(
                Platform::NoFluffJobs,
                "n2",
                "Python data scientist",
            ))
            .await
            .unwrap()
            .id();

        let mut first = vec![0.0f32; TEST_DIM];
        first[0] = 1.0;
        store
            .upsert_batch(&[id1], &[first], &["first".to_string()])
            .await
            .unwrap();
        let mut other = vec![0.0f32; TEST_DIM];
        other[2] = 1.0;
        store
            .upsert_batch(&[id2], &[other], &["other".to_string()])
            .await
            .unwrap();

        // overwrite id1 embedding to align with dimension 1 instead of 0
        let mut second = vec![0.0f32; TEST_DIM];
        second[1] = 1.0;
        store
            .upsert_batch(&[id1], &[second], &["first".to_string()])
            .await
            .unwrap();

        let mut query = vec![0.0f32; TEST_DIM];
        query[1] = 1.0;
        let result = store
            .search("zzz", &query, &[id1, id2], 10, 0)
            .await
            .unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(
            result.items[0].0, id1,
            "overwritten embedding should rank id1 first for query[1]"
        );

        let mut wrong_query = vec![0.0f32; TEST_DIM];
        wrong_query[2] = 1.0;
        let wrong_result = store
            .search("zzz", &wrong_query, &[id1, id2], 10, 0)
            .await
            .unwrap();
        assert_eq!(wrong_result.total, 2);
        assert_eq!(
            wrong_result.items[0].0, id2,
            "old embedding should be gone: id1 must not rank first for query[2]"
        );
    }
}
