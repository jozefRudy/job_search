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

    pub async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let () = std::future::ready(()).await;
        Ok(texts.iter().map(|t| embedding_for(t, self.dim)).collect())
    }

    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let () = std::future::ready(()).await;
        Ok(embedding_for(text, self.dim))
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

    #[tokio::test]
    async fn embed_is_deterministic_and_correct_dim() {
        let e = Embedder::new(DEFAULT_EMBEDDING_MODEL, EMBEDDING_DIM);
        let a = e.embed("rust backend role").await.unwrap();
        let b = e.embed("rust backend role").await.unwrap();
        assert_eq!(a.len(), EMBEDDING_DIM);
        assert_eq!(a, b);

        let c = e.embed("different text").await.unwrap();
        assert_ne!(a, c);
    }

    #[tokio::test]
    async fn embed_batch_matches_individual() {
        let e = Embedder::new(DEFAULT_EMBEDDING_MODEL, 16);
        let batch = e.embed_batch(&["a", "b", "c"]).await.unwrap();
        let a = e.embed("a").await.unwrap();
        let b = e.embed("b").await.unwrap();
        let c = e.embed("c").await.unwrap();
        assert_eq!(batch, vec![a, b, c]);
    }
}
