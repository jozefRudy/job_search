//! Thin domain wrapper over `patterns::embed`: nomic model choice + its
//! query/document prefixes, baked into the `Embedder` at load time.
//! Machinery (loading, batching, prefix application, fake) lives in patterns.

use std::path::Path;

use anyhow::Result;

pub use patterns::embed::Embedder;

use patterns::embed::{LoadOptions, ModelSpec, Pooling, Prefixes};

/// Hub repo of the default embedding model.
pub const MODEL: &str = "nomic-ai/nomic-embed-text-v1.5";
const REVISION: &str = "e9b6763023c676ca8431644204f50c2b100d9aab";
const FILE: &str = "onnx/model.onnx";
const NATIVE_DIM: usize = 768;

/// Domain helper: the pinned nomic spec — 768-d mean-pooled `last_hidden_state` with the
/// model's `search_query: `/`search_document: ` prefixes baked in (symmetric models:
/// `Prefixes::none()`).
fn model_spec() -> Result<ModelSpec> {
    Ok(ModelSpec::new(MODEL, REVISION, FILE, NATIVE_DIM)?
        .with_pooling(Pooling::Mean)
        .with_prefixes(&Prefixes::new("search_query: ", "search_document: ")))
}

pub async fn load_default(cache_dir: &Path) -> Result<Embedder> {
    let options = LoadOptions::new(model_spec()?).with_intra_threads(4);
    Embedder::load(options, cache_dir).await
}

#[cfg(test)]
mod tests {
    use super::*;

    static EMBEDDER: tokio::sync::OnceCell<Embedder> = tokio::sync::OnceCell::const_new();

    async fn test_embedder() -> &'static Embedder {
        EMBEDDER
            .get_or_init(|| async { load_default(&test_cache_dir()).await.unwrap() })
            .await
    }

    fn test_cache_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("jobsearch_embed_tests");
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    #[ignore = "downloads model"]
    async fn load_returns_expected_dim() {
        let embedder = test_embedder().await;
        assert_eq!(embedder.dim(), 768);
    }

    #[tokio::test]
    #[ignore = "downloads model"]
    async fn real_embedding_is_deterministic() {
        let embedder = test_embedder().await;
        let a = embedder.embed_query("rust backend role").await.unwrap();
        let b = embedder.embed_query("rust backend role").await.unwrap();
        assert_eq!(a, b);
    }

    #[tokio::test]
    #[ignore = "downloads model"]
    async fn real_similar_texts_score_higher() {
        let embedder = test_embedder().await;
        let docs = [
            "rust backend developer".to_string(),
            "senior rust engineer".to_string(),
            "python data scientist".to_string(),
        ];
        let mut opts = embedder.default_chunk_options();
        opts.min_tokens = 1; // embed short texts too (as the indexer does)
        let rows = embedder
            .embed_batch_document_chunks(&docs, &opts)
            .await
            .unwrap();
        let first = |doc_ix: usize| {
            rows.iter()
                .find(|r| r.doc_ix == doc_ix && r.chunk_ix == 0)
                .expect("first chunk")
                .embedding
                .clone()
        };
        let (a, b, c) = (first(0), first(1), first(2));

        let sim_close = cosine_similarity(&a, &b);
        let sim_far = cosine_similarity(&a, &c);
        assert!(sim_close > sim_far, "similar texts should score higher");
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        dot / (norm_a * norm_b)
    }
}
