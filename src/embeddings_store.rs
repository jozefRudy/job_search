use anyhow::Result;
use std::path::Path;

use crate::db::Db;
use crate::embed::{EMBEDDING_DIM, Embedder};
use crate::models::Job;

pub struct EmbeddingsStore {
    _db: Db,
    _uri: String,
    _embedder: Embedder,
    _dim: usize,
}

impl EmbeddingsStore {
    pub async fn open(
        _sqlite_db_path: &Path,
        _model_id: &'static str,
        db: Db,
        embedder: Embedder,
    ) -> Result<Self> {
        let () = std::future::ready(()).await;
        Ok(Self {
            _db: db,
            _uri: String::new(),
            _embedder: embedder,
            _dim: EMBEDDING_DIM,
        })
    }

    pub async fn upsert_embedding(&self, _job_id: i64, _embedding: &[f32]) -> Result<()> {
        todo!()
    }

    pub async fn upsert_batch(&self, _job_ids: &[i64], _embeddings: &[Vec<f32>]) -> Result<()> {
        todo!()
    }

    pub async fn get_unvectorized_jobs(&self, _limit: i64) -> Result<Vec<Job>> {
        todo!()
    }

    pub async fn search(
        &self,
        _embedding: &[f32],
        _candidate_ids: &[i64],
        _limit: usize,
        _offset: usize,
    ) -> Result<Vec<(i64, f32)>> {
        todo!()
    }

    pub async fn maintenance(&self) -> Result<()> {
        todo!()
    }

    pub async fn index_unvectorized(&self, _batch_size: usize) -> Result<usize> {
        todo!()
    }
}
