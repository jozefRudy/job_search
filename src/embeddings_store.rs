// TODO: implement minimal Lance embeddings store for jobs
// - borrow simple Lance patterns from src/vector_db/reddit_store.rs:
//     - Arrow schema + FixedSizeList embedding field
//     - Dataset::open / Dataset::write with RecordBatch
//     - scanner.nearest() + scanner.filter() + distance metric
//     - create_index_builder for IVF_HNSW_PQ vector index
//     - compaction + optimize_indices as a `maintenance()` method called after bulk upserts
// - do NOT borrow: background actor, batching, periodic flush, multi-write
// - one dataset per model: data_dir/embeddings-{model}.lance
// - schema: job_id (Int64), embedding (FixedSizeList<Float32, DIM>), created_at (Timestamp)
// - upsert by job_id with MergeInsertBuilder:
//     - key on `job_id`
//     - WhenMatched::UpdateAll, WhenNotMatched::InsertAll
//     - execute with a RecordBatch of (job_id, embedding, created_at)
//     - run maintenance() periodically to compact fragments
// - search flow:
//     1. SQLite pre-filter with CommonListArgs (no pagination) -> candidate job_ids
//     2. Lance vector search restricted to those ids with limit/offset -> ranked page of ids
//     3. SQLite fetch full job rows for those ids
// - search requires Sort::Relevance; strict backend: reject search with sort != Relevance
// - frontend sets sort=relevance in URL when search query is active; sort dropdown reads URL
// - delete or ignore src/vector_db/reddit_store.rs; it is reference only
