use anyhow::Result;

pub const DEFAULT_EMBEDDING_MODEL: &str = "fake";
pub const EMBEDDING_DIM: usize = 384;

pub struct Embedder {
    pub model_id: &'static str,
    pub dim: usize,
}

impl Embedder {
    #[must_use]
    pub fn new(model_id: &'static str, dim: usize) -> Self {
        Self { model_id, dim }
    }

    pub async fn embed_batch(&self, _texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        todo!()
    }

    pub async fn embed(&self, _text: &str) -> Result<Vec<f32>> {
        todo!()
    }
}
