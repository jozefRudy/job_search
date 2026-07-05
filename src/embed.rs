// TODO: implement fake embedding module
// - model_id: &'static str set on startup
// - struct Embedder { model_id: &'static str, dim: usize }
// - impl Embedder:
//     - new(model_id: &'static str, dim: usize) -> Self
//     - async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>
//         returns deterministic Vec<f32> of length `dim` per input
//     - async fn embed(&self, text: &str) -> Result<Vec<f32>>
// - enough to wire up search/DB plumbing; replace with real candle model later
