use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use arrow::array::{Float32Array, RecordBatch, RecordBatchIterator, as_primitive_array};
use arrow::datatypes::{DataType, Field, Float32Type, Int64Type, Schema, TimeUnit};
use arrow_array::builder::{
    FixedSizeListBuilder, Float32Builder, Int64Builder, TimestampMicrosecondBuilder,
};
use chrono::Utc;
use futures::TryStreamExt;
use lance::dataset::optimize::{CompactionOptions, compact_files};
use lance::dataset::{Dataset, WriteMode, WriteParams};
use lance::index::DatasetIndexExt;
use lance::index::vector::VectorIndexParams;
use lance_index::IndexType;
use lance_index::optimize::OptimizeOptions;
use lance_index::vector::{hnsw::builder::HnswBuildParams, ivf::IvfBuildParams, pq::PQBuildParams};
use lance_linalg::distance::DistanceType;

use crate::db::Db;
use crate::embed::Embedder;
use crate::models::Job;
use tokio::sync::Mutex;

pub struct EmbeddingsStore {
    db: Db,
    uri: String,
    embedder: Embedder,
    dim: usize,
    dataset: Mutex<Dataset>,
}

impl EmbeddingsStore {
    pub async fn open(
        sqlite_db_path: &Path,
        model_id: &'static str,
        db: Db,
        embedder: Embedder,
    ) -> Result<Self> {
        let base = sqlite_db_path.parent().unwrap_or_else(|| Path::new("."));
        tokio::fs::create_dir_all(base).await?;
        let model_dir = model_id.replace('/', "-");
        let lance_dir = base.join("lance");
        tokio::fs::create_dir_all(&lance_dir).await?;
        let uri = lance_dir
            .join(format!("embeddings-{model_dir}"))
            .to_string_lossy()
            .to_string();
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

        Ok(Self {
            db,
            uri,
            embedder,
            dim,
            dataset: Mutex::new(dataset),
        })
    }

    #[must_use]
    pub fn embedder(&self) -> &Embedder {
        &self.embedder
    }

    pub async fn upsert_embedding(&self, job_id: i64, embedding: &[f32]) -> Result<()> {
        self.upsert_batch(&[job_id], &[embedding.to_vec()]).await
    }

    pub async fn upsert_batch(&self, job_ids: &[i64], embeddings: &[Vec<f32>]) -> Result<()> {
        let batch = embeddings_to_batch(job_ids, embeddings, self.dim)?;
        let schema = batch.schema();
        let reader = RecordBatchIterator::new(vec![Ok(batch)], schema);
        let params = WriteParams {
            mode: WriteMode::Append,
            ..Default::default()
        };
        let new_dataset = Dataset::write(reader, &self.uri, Some(params))
            .await
            .context("appending embeddings")?;
        *self.dataset.lock().await = new_dataset;
        Ok(())
    }

    pub async fn get_unvectorized_jobs(&self, limit: i64) -> Result<Vec<Job>> {
        let vectorized_ids = self.list_vectorized_ids().await?;
        let ids = self.db.get_job_ids_except(&vectorized_ids, limit).await?;
        self.db.get_jobs(&ids).await
    }

    pub async fn search(
        &self,
        embedding: &[f32],
        candidate_ids: &[i64],
        limit: usize,
        offset: usize,
    ) -> Result<Vec<(i64, f32)>> {
        if candidate_ids.is_empty() || embedding.len() != self.dim {
            return Ok(Vec::new());
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

        let mut scanner = dataset.scan();
        scanner.filter(&format!("job_id IN ({id_list})"))?;
        scanner.prefilter(true);
        scanner.distance_metric(DistanceType::Cosine);
        let query_arr = Float32Array::from(embedding.to_vec());
        let top_n = limit.saturating_add(offset);
        scanner.nearest("embedding", &query_arr, top_n)?;
        scanner.project(&["job_id", "_distance"])?;

        let batches: Vec<RecordBatch> = scanner.try_into_stream().await?.try_collect().await?;
        let mut results = Vec::new();
        for batch in &batches {
            let job_ids = as_primitive_array::<Int64Type>(
                batch.column_by_name("job_id").context("job_id column")?,
            );
            let distances = as_primitive_array::<Float32Type>(
                batch
                    .column_by_name("_distance")
                    .context("_distance column")?,
            );
            for i in 0..batch.num_rows() {
                let id = job_ids.value(i);
                let distance = distances.value(i);
                let similarity = 1.0 - distance / 2.0;
                results.push((id, similarity));
            }
        }

        Ok(results.into_iter().skip(offset).take(limit).collect())
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
            self.upsert_batch(&ids, &embeddings).await?;
            total += ids.len();
            on_progress(total);
        }
        self.maintenance().await?;
        Ok(total)
    }

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
    dim: usize,
) -> Result<RecordBatch> {
    if job_ids.len() != embeddings.len() {
        bail!("job_ids and embeddings must have the same length");
    }

    let mut id_builder = Int64Builder::new();
    let mut emb_builder = FixedSizeListBuilder::new(
        Float32Builder::new(),
        i32::try_from(dim).expect("embedding dimension fits in i32"),
    );
    let mut ts_builder = TimestampMicrosecondBuilder::new();
    let now = Utc::now().timestamp_micros();

    for (id, emb) in job_ids.iter().zip(embeddings.iter()) {
        if emb.len() != dim {
            bail!("expected embedding dimension {dim}, got {}", emb.len());
        }
        id_builder.append_value(*id);
        emb_builder.values().append_slice(emb);
        emb_builder.append(true);
        ts_builder.append_value(now);
    }

    let schema = Arc::new(arrow_schema(dim));
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id_builder.finish()),
            Arc::new(emb_builder.finish()),
            Arc::new(ts_builder.finish()),
        ],
    )?;
    Ok(batch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embed::DEFAULT_EMBEDDING_MODEL;
    const TEST_DIM: usize = 768;
    use crate::models::{
        Data, EfinancialcareersJobDetail, LinkedInJobDetail, NoFluffJobDetail, Platform, Rating,
        UpworkJobDetail,
    };

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
            Platform::LinkedIn => Data::LinkedIn {
                detail: LinkedInJobDetail::default(),
            },
        };
        Job {
            id: 0,
            platform,
            external_id: external_id.to_string(),
            title: title.to_string(),
            description: None,
            url: format!("https://example.com/{external_id}"),
            budget: None,
            tags: vec![],
            raw,
            company: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            rating: Rating::Neutral,
            note: None,
            applied_at: None,
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
        let dataset = Dataset::open(&store.uri).await.unwrap();
        assert_eq!(dataset.schema().fields.len(), 3);
    }

    #[tokio::test]
    async fn open_uses_lance_subdirectory() {
        let (tmp, _db, store) = test_store().await;
        let expected = tmp
            .path()
            .join("lance")
            .join("embeddings-nomic-ai-nomic-embed-text-v1.5");
        assert_eq!(store.uri, expected.to_string_lossy().to_string());
    }

    #[tokio::test]
    async fn index_unvectorized_embeddings() {
        let (_tmp, db, store) = test_store().await;
        db.upsert_job(&test_job(Platform::Upwork, "u1", "Rust backend developer"))
            .await
            .unwrap();
        db.upsert_job(&test_job(
            Platform::NoFluffJobs,
            "n1",
            "Senior Python engineer",
        ))
        .await
        .unwrap();

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
            .unwrap();
        let id2 = db
            .upsert_job(&test_job(
                Platform::NoFluffJobs,
                "n2",
                "Python data scientist",
            ))
            .await
            .unwrap();

        let mut query = vec![0.0f32; TEST_DIM];
        query[0] = 1.0;
        let mut emb1 = vec![0.0f32; TEST_DIM];
        emb1[0] = 1.0;
        let mut emb2 = vec![0.0f32; TEST_DIM];
        emb2[1] = 1.0;
        store
            .upsert_batch(&[id1, id2], &[emb1, emb2])
            .await
            .unwrap();

        let ranked = store.search(&query, &[id1, id2], 10, 0).await.unwrap();
        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].0, id1);
    }

    #[tokio::test]
    async fn search_respects_candidate_filter() {
        let (_tmp, db, store) = test_store().await;
        let id1 = db
            .upsert_job(&test_job(Platform::Upwork, "u1", "Rust backend developer"))
            .await
            .unwrap();
        let id2 = db
            .upsert_job(&test_job(
                Platform::NoFluffJobs,
                "n2",
                "Python data scientist",
            ))
            .await
            .unwrap();

        let mut query = vec![0.0f32; TEST_DIM];
        query[0] = 1.0;
        let mut emb1 = vec![0.0f32; TEST_DIM];
        emb1[0] = 1.0;
        let mut emb2 = vec![0.0f32; TEST_DIM];
        emb2[1] = 1.0;
        store
            .upsert_batch(&[id1, id2], &[emb1, emb2])
            .await
            .unwrap();

        let ranked = store.search(&query, &[id2], 10, 0).await.unwrap();
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].0, id2);
    }

    #[tokio::test]
    async fn multiple_consecutive_searches_after_write() {
        let (_tmp, db, store) = test_store().await;
        let id1 = db
            .upsert_job(&test_job(Platform::Upwork, "u1", "Rust backend developer"))
            .await
            .unwrap();

        let mut emb1 = vec![0.0f32; TEST_DIM];
        emb1[0] = 1.0;
        store.upsert_batch(&[id1], &[emb1]).await.unwrap();

        let mut query = vec![0.0f32; TEST_DIM];
        query[0] = 1.0;
        for i in 0..30 {
            let ranked = store.search(&query, &[id1], 10, 0).await.unwrap();
            assert_eq!(ranked.len(), 1, "search {i} should return one result");
            assert_eq!(ranked[0].0, id1);
        }
    }
}
