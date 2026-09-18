//! Backend contracts. Candle/ONNX embedders and Qdrant/pgvector stores
//! implement these traits; local deterministic backends in `embed` keep the
//! system fully functional offline today.

use async_trait::async_trait;

/// Dense text embedding backend.
#[async_trait]
pub trait Embedder: Send + Sync {
    fn dimension(&self) -> usize;
    async fn embed(&self, texts: &[String]) -> crate::Result<Vec<Vec<f32>>>;
}

/// Vector similarity backend.
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Upsert `(id, vector)` pairs.
    async fn upsert(&mut self, items: Vec<(String, Vec<f32>)>) -> crate::Result<()>;
    /// Top-k cosine matches as `(id, score)`.
    async fn search(&self, vector: &[f32], k: usize) -> crate::Result<Vec<(String, f32)>>;
}

pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "embedding dimension mismatch");
    let (mut dot, mut na, mut nb) = (0.0f32, 0.0f32, 0.0f32);
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_identity_and_orthogonality() {
        assert!((cosine(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert!(cosine(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-6);
        assert_eq!(cosine(&[0.0, 0.0], &[1.0, 1.0]), 0.0);
    }
}
