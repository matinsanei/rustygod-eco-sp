//! Deterministic local backends: no model downloads, no network.
//! A Candle/ONNX `Embedder` and Qdrant `VectorStore` slot into the same
//! traits when the GPU/download milestone lands.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::traits::{cosine, Embedder, VectorStore};

/// Fixed-key token-hash embedder (dim 256, L2-normalized).
/// Deterministic across runs and processes — suitable as the v1 local
/// backend and as a test double for the vector tier.
pub struct HashingEmbedder {
    dim: usize,
}

impl HashingEmbedder {
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }

    fn token_vec(&self, token: &str) -> Vec<u64> {
        // Character trigrams (fastText-style): shared substrings drive
        // similarity, so related words collide and unrelated ones don't.
        let padded = format!("<{token}>");
        let chars: Vec<char> = padded.chars().collect();
        let mut out = Vec::new();
        if chars.len() < 3 {
            return vec![hash_with_seed(token, 0), hash_with_seed(token, 1)];
        }
        for w in chars.windows(3) {
            let tri: String = w.iter().collect();
            out.push(hash_with_seed(&tri, 0));
            out.push(hash_with_seed(&tri, 0x9e3779b9));
        }
        out
    }
}

fn hash_with_seed(s: &str, seed: u64) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    seed.hash(&mut h);
    s.hash(&mut h);
    h.finish()
}

impl Default for HashingEmbedder {
    fn default() -> Self {
        Self::new(256)
    }
}

#[async_trait::async_trait]
impl Embedder for HashingEmbedder {
    fn dimension(&self) -> usize {
        self.dim
    }

    async fn embed(&self, texts: &[String]) -> crate::Result<Vec<Vec<f32>>> {
        Ok(texts
            .iter()
            .map(|t| {
                let mut v = vec![0.0f32; self.dim];
                for token in t.to_lowercase().split_whitespace() {
                    for (i, word) in self.token_vec(token).iter().enumerate() {
                        let idx = (*word as usize) % self.dim;
                        v[idx] += 1.0 + (i as f32 * 0.017);
                    }
                }
                let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
                if norm > 0.0 {
                    for x in v.iter_mut() {
                        *x /= norm;
                    }
                }
                v
            })
            .collect())
    }
}

/// Brute-force in-memory cosine store. Correctness baseline for the
/// Qdrant/pgvector backends (same scores within float tolerance).
#[derive(Default)]
pub struct InMemoryVectorStore {
    items: HashMap<String, Vec<f32>>,
}

#[async_trait::async_trait]
impl VectorStore for InMemoryVectorStore {
    async fn upsert(&mut self, items: Vec<(String, Vec<f32>)>) -> crate::Result<()> {
        for (id, v) in items {
            self.items.insert(id, v);
        }
        Ok(())
    }

    async fn search(&self, vector: &[f32], k: usize) -> crate::Result<Vec<(String, f32)>> {
        let mut scored: Vec<(String, f32)> = self
            .items
            .iter()
            .map(|(id, v)| (id.clone(), cosine(v, vector)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);
        Ok(scored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn embedder_is_deterministic_and_normalized() {
        let e = HashingEmbedder::default();
        assert_eq!(e.dimension(), 256);
        let a = e.embed(&["red cotton shirt".to_string()]).await.unwrap();
        let b = e.embed(&["red cotton shirt".to_string()]).await.unwrap();
        assert_eq!(a, b);
        let norm: f32 = a[0].iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn similar_texts_score_higher() {
        let e = HashingEmbedder::default();
        let vs = e
            .embed(&[
                "red cotton shirt".to_string(),
                "red cotton shirt large".to_string(),
                "stainless steel wrench".to_string(),
            ])
            .await
            .unwrap();
        let q = e.embed(&["cotton shirt".to_string()]).await.unwrap();
        let s_close = cosine(&vs[1], &q[0]);
        let s_far = cosine(&vs[2], &q[0]);
        assert!(s_close > s_far, "{s_close} should beat {s_far}");
    }

    #[tokio::test]
    async fn store_top_k_orders_by_score() {
        let e = HashingEmbedder::default();
        let texts = ["apple".to_string(), "apple pie".to_string(), "engine block".to_string()];
        let vs = e.embed(&texts).await.unwrap();
        let mut store = InMemoryVectorStore::default();
        store
            .upsert(
                ["a".to_string(), "b".to_string(), "c".to_string()]
                    .into_iter()
                    .zip(vs)
                    .collect(),
            )
            .await
            .unwrap();
        let q = e.embed(&["apple".to_string()]).await.unwrap();
        let hits = store.search(&q[0], 2).await.unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].0, "a");
        assert!(hits[0].1 >= hits[1].1);
    }
}
