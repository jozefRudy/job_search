// TODO: implement Lance embeddings store for jobs
// - borrow simple Lance patterns from src/vector_db/reddit_store.rs:
//     - Arrow schema + FixedSizeList embedding field
//     - Dataset::open / Dataset::write with RecordBatch
//     - scanner.nearest() + scanner.filter() + distance metric
//     - create_index_builder for IVF_HNSW_PQ vector index
//     - compaction + optimize_indices as a `maintenance()` method called after bulk upserts
// - do NOT borrow the Lance write actor pattern from reddit_store:
//     - no mpsc command enum, no command handler, no dedicated task serializing Lance writes
// - server-only background vectorizer:
//     - spawned by EmbeddingsStore::spawn_vectorizer() when server starts
//     - tokio task loop: every 30s, fetch up to BATCH unvectorized jobs via Db::get_job_ids_except,
//       embed batch via Embedder::embed_batch, await upsert_batch, run maintenance() after bulk
//     - shutdown() signals stop and awaits final batch flush
// - EmbeddingsStore struct holding the SQLite Db, Lance dataset URI, and embedder
// - methods:
//     - open(sqlite_db_path, model_id, db, embedder) -> open dataset at embeddings-{model}.lance next to the SQLite file
//     - async upsert_embedding(job_id: i64, embedding: &[f32]) -> MergeInsertBuilder upsert into Lance
//     - async upsert_batch(job_ids: &[i64], embeddings: &[Vec<f32>]) -> MergeInsertBuilder upsert into Lance
//     - async get_unvectorized_jobs(limit: i64) -> Result<Vec<Job>>:
//         - fetch all job_ids from the Lance dataset
//         - call Db::get_job_ids_except(vectorized_ids, limit) -> Vec<i64>
//         - call Db::get_jobs(ids) -> Vec<Job> for those ids
//     - async search(embedding: &[f32], candidate_ids: &[i64], limit: usize, offset: usize) -> Result<Vec<(i64, f32)>>
//         - Lance nearest-neighbor restricted to candidate ids
//     - async maintenance() -> compact + optimize_indices
//     - fn spawn_vectorizer(self: Arc<Self>) -> VectorizerHandle
//     - async fn shutdown(handle: VectorizerHandle) -> stop vectorizer and flush current batch
// - one dataset per model: reuse the directory containing the SQLite DB, filename embeddings-{model}.lance
// - schema: job_id (Int64), embedding (FixedSizeList<Float32, DIM>), created_at (Timestamp)
// - upsert by job_id with MergeInsertBuilder:
//     - key on `job_id`
//     - WhenMatched::UpdateAll, WhenNotMatched::InsertAll
//     - execute with a RecordBatch of (job_id, embedding, created_at)
//     - run maintenance() periodically after bulk upserts
// - search flow:
//     1. Db::filter_job_ids(filter: &JobFilter) -> Vec<i64> -
//        SQLite pre-filter using CommonListArgs (no pagination), returns candidate ids only
//     2. EmbeddingsStore::search(embedding, candidate_ids, limit, offset) -> Vec<(job_id, score)> -
//        Lance nearest-neighbor restricted to those ids; returns the ranked page of ids
//     3. Db::get_jobs(ids) -> Vec<Job> -
//        fetch full rows for exactly those ids, preserving the Lance order
//        (no extra filter/limit; the page is already determined by step 2)
// - relies on Db methods in src/db.rs:
//     - get_job_ids_except(excluded_ids, limit) for finding jobs not yet in the Lance dataset
//     - filter_job_ids(filter) for pre-filtering candidate ids
//     - get_jobs(ids) preserving input order for fetching the final ranked page
// - add Db::get_job_ids_except(excluded_ids: &[i64], limit: i64) -> Result<Vec<i64>> in src/db.rs
//     - SQL: SELECT id FROM jobs WHERE id NOT IN (SELECT value FROM json_each(?1)) ORDER BY created_at ASC LIMIT ?2
// - add Db::filter_job_ids(filter: &JobFilter) -> Result<Vec<i64>> in src/db.rs
//     - same WHERE clauses as list_jobs_filtered, but SELECT j.id only and no LIMIT/OFFSET
// - add Sort::Relevance variant to src/models.rs; search requires it and rejects other sorts
// - frontend sets sort=relevance in URL when search query is active; sort dropdown reads URL
// - delete or ignore src/vector_db/reddit_store.rs; it is reference only
