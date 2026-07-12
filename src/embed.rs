use anyhow::Result;
use candle_core::{DType, Device, Module, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::jina_bert::{BertModel, Config};
use hf_hub::{HFClient, split_id};
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};

pub const DEFAULT_EMBEDDING_MODEL: &str = "jinaai/jina-embeddings-v2-base-en";
pub const MAX_SEQ_LEN: usize = 2048;

pub struct ModelState {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
    dim: usize,
}

pub enum Embedder {
    Fake { dim: usize },
    Jina(Box<ModelState>),
}

impl Embedder {
    pub async fn load(cache_dir: &std::path::Path) -> Result<Self> {
        let client = HFClient::builder()
            .cache_dir(cache_dir.to_path_buf())
            .build()?;
        let (owner, name) = split_id(DEFAULT_EMBEDDING_MODEL);
        let repo = client.model(owner, name);

        let config_path = repo.download_file().filename("config.json").send().await?;
        let tokenizer_path = repo
            .download_file()
            .filename("tokenizer.json")
            .send()
            .await?;
        let model_path = repo
            .download_file()
            .filename("model.safetensors")
            .send()
            .await?;

        let config: Config = serde_json::from_str(&tokio::fs::read_to_string(&config_path).await?)?;
        let mut tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow::anyhow!("failed to load tokenizer: {e}"))?;
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: MAX_SEQ_LEN,
                ..Default::default()
            }))
            .map_err(|e| anyhow::anyhow!("failed to set truncation: {e}"))?;
        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::BatchLongest,
            ..Default::default()
        }));

        let device = default_device();
        let vb =
            unsafe { VarBuilder::from_mmaped_safetensors(&[model_path], DType::F32, &device)? };
        let model = BertModel::new(vb, &config)?;
        let dim = config.hidden_size;

        Ok(Self::Jina(Box::new(ModelState {
            model,
            tokenizer,
            device,
            dim,
        })))
    }

    #[must_use]
    pub fn fake(dim: usize) -> Self {
        Self::Fake { dim }
    }

    #[must_use]
    pub fn dim(&self) -> usize {
        match self {
            Self::Fake { dim } => *dim,
            Self::Jina(state) => state.dim,
        }
    }

    pub async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        match self {
            Self::Fake { dim } => {
                let () = std::future::ready(()).await;
                Ok(texts.iter().map(|t| embedding_for(t, *dim)).collect())
            }
            Self::Jina(state) => embed_jina(state, texts),
        }
    }

    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let mut batch = self.embed_batch(&[text]).await?;
        batch
            .pop()
            .ok_or_else(|| anyhow::anyhow!("expected one embedding, got none"))
    }
}

fn embed_jina(state: &ModelState, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
    let encodings = state
        .tokenizer
        .encode_batch(texts.to_vec(), true)
        .map_err(|e| anyhow::anyhow!("tokenization failed: {e}"))?;

    let token_ids: Vec<Tensor> = encodings
        .iter()
        .map(|e| Tensor::new(e.get_ids(), &state.device))
        .collect::<candle_core::Result<Vec<_>>>()?;
    let input_ids = Tensor::stack(&token_ids, 0)?;

    let attention_masks: Vec<Tensor> = encodings
        .iter()
        .map(|e| Tensor::new(e.get_attention_mask(), &state.device))
        .collect::<candle_core::Result<Vec<_>>>()?;
    let attention_mask = Tensor::stack(&attention_masks, 0)?;

    let outputs = state.model.forward(&input_ids)?;
    let pooled = mean_pool(&outputs, &attention_mask)?;
    let normalized = l2_normalize(&pooled)?;
    Ok(normalized.to_vec2::<f32>()?)
}

fn mean_pool(output: &Tensor, mask: &Tensor) -> Result<Tensor> {
    let mask = mask.to_dtype(DType::F32)?;
    let mask = mask.unsqueeze(2)?;
    let masked = output.broadcast_mul(&mask)?;
    let sum = masked.sum(&[1usize][..])?;
    let mask_sum = mask.sum(&[1usize][..])?;
    Ok(sum.broadcast_div(&mask_sum)?)
}

fn l2_normalize(x: &Tensor) -> Result<Tensor> {
    Ok(x.broadcast_div(&x.sqr()?.sum_keepdim(1)?.sqrt()?)?)
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

fn default_device() -> Device {
    try_metal().unwrap_or(Device::Cpu)
}

#[cfg(feature = "metal")]
fn try_metal() -> Option<Device> {
    Device::new_metal(0).ok()
}

#[cfg(not(feature = "metal"))]
fn try_metal() -> Option<Device> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

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

    static EMBEDDER: tokio::sync::OnceCell<Embedder> = tokio::sync::OnceCell::const_new();

    async fn test_embedder() -> &'static Embedder {
        EMBEDDER
            .get_or_init(|| async { Embedder::load(&test_cache_dir()).await.unwrap() })
            .await
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
        let a = embedder.embed("rust backend developer").await.unwrap();
        let b = embedder.embed("senior rust engineer").await.unwrap();
        let c = embedder.embed("python data scientist").await.unwrap();

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
