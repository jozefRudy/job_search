use std::sync::{Arc, Mutex};

use anyhow::Result;
use fastembed::{EmbeddingModel, ExecutionProviderDispatch, TextEmbedding, TextInitOptions};
use ort::ep::cpu::CPU;

pub const DEFAULT_EMBEDDING_MODEL: &str = "nomic-ai/nomic-embed-text-v1.5";

pub enum Embedder {
    Fake {
        dim: usize,
    },
    FastEmbed {
        model: Arc<Mutex<TextEmbedding>>,
        dim: usize,
    },
}

impl Embedder {
    pub async fn load(base_dir: &std::path::Path) -> Result<Self> {
        let cache_dir = base_dir.join("models");
        tokio::fs::create_dir_all(&cache_dir).await?;
        let model = tokio::task::spawn_blocking(move || {
            let options = TextInitOptions::new(EmbeddingModel::NomicEmbedTextV15)
                .with_cache_dir(cache_dir)
                .with_show_download_progress(true)
                .with_execution_providers(vec![ExecutionProviderDispatch::from(CPU::default())]);
            TextEmbedding::try_new(options)
        })
        .await??;

        let dim = TextEmbedding::get_model_info(&EmbeddingModel::NomicEmbedTextV15)?.dim;

        Ok(Self::FastEmbed {
            model: Arc::new(Mutex::new(model)),
            dim,
        })
    }

    #[must_use]
    pub fn fake(dim: usize) -> Self {
        Self::Fake { dim }
    }

    #[must_use]
    pub fn dim(&self) -> usize {
        match self {
            Self::Fake { dim } | Self::FastEmbed { dim, .. } => *dim,
        }
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        match self {
            Self::Fake { dim } => {
                let () = std::future::ready(()).await;
                Ok(texts.iter().map(|t| embedding_for(t, *dim)).collect())
            }
            Self::FastEmbed { model, .. } => {
                let model = Arc::clone(model);
                let texts: Vec<String> = texts.iter().map(|s| (*s).to_string()).collect();
                tokio::task::spawn_blocking(move || {
                    let mut model = model.lock().unwrap();
                    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
                    model.embed(&refs, None)
                })
                .await?
            }
        }
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let mut batch = self.embed_batch(&[text]).await?;
        batch
            .pop()
            .ok_or_else(|| anyhow::anyhow!("expected one embedding, got none"))
    }

    pub async fn embed_query(&self, text: &str) -> Result<Vec<f32>> {
        self.embed(&format!("search_query: {text}")).await
    }

    pub async fn embed_document(&self, text: &str) -> Result<Vec<f32>> {
        self.embed(&format!("search_document: {text}")).await
    }

    pub async fn embed_batch_documents(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let prefixed: Vec<String> = texts
            .iter()
            .map(|text| format!("search_document: {text}"))
            .collect();
        let refs: Vec<&str> = prefixed.iter().map(String::as_str).collect();
        self.embed_batch(&refs).await
    }
}

fn fnv1a(text: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

fn splitmix64(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *seed;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn embedding_for(text: &str, dim: usize) -> Vec<f32> {
    let mut seed = fnv1a(text);
    (0..dim)
        .map(|_| {
            let v = splitmix64(&mut seed);
            (v as f32 / u64::MAX as f32).mul_add(2.0, -1.0)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    static EMBEDDER: tokio::sync::OnceCell<Embedder> = tokio::sync::OnceCell::const_new();

    async fn test_embedder() -> &'static Embedder {
        EMBEDDER
            .get_or_init(|| async { Embedder::load(&test_cache_dir()).await.unwrap() })
            .await
    }

    #[tokio::test]
    async fn embed_is_deterministic_and_correct_dim() {
        const TEST_DIM: usize = 768;
        let e = Embedder::fake(TEST_DIM);
        let a = e.embed("rust backend role").await.unwrap();
        let b = e.embed("rust backend role").await.unwrap();
        assert_eq!(a.len(), TEST_DIM);
        assert_eq!(a, b);

        let c = e.embed("different text").await.unwrap();
        assert_ne!(a, c);
    }

    #[tokio::test]
    async fn embed_batch_matches_individual() {
        let e = Embedder::fake(16);
        let batch = e.embed_batch(&["a", "b", "c"]).await.unwrap();
        let a = e.embed("a").await.unwrap();
        let b = e.embed("b").await.unwrap();
        let c = e.embed("c").await.unwrap();
        assert_eq!(batch, vec![a, b, c]);
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
        let a = embedder.embed("rust backend role").await.unwrap();
        let b = embedder.embed("rust backend role").await.unwrap();
        assert_eq!(a, b);
    }

    #[tokio::test]
    #[ignore = "downloads model"]
    async fn real_similar_texts_score_higher() {
        let embedder = test_embedder().await;
        let a = embedder
            .embed_document("rust backend developer")
            .await
            .unwrap();
        let b = embedder
            .embed_document("senior rust engineer")
            .await
            .unwrap();
        let c = embedder
            .embed_document("python data scientist")
            .await
            .unwrap();

        let sim_close = cosine_similarity(&a, &b);
        let sim_far = cosine_similarity(&a, &c);
        assert!(sim_close > sim_far, "similar texts should score higher");
    }

    fn test_cache_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("jobsearch_embed_tests");
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        dot / (norm_a * norm_b)
    }
}
