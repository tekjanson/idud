//! PHASE 3.3: The Embedding Layer
//! Generate semantic embeddings for signatories
//! Pre-compute once, reuse forever (zero-token traversal)

use crate::types::Signatory;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub signatory_id: String,
    pub vector: Vec<f32>,
    pub computed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingIndex {
    pub embeddings: HashMap<String, Vec<f32>>,
    pub metadata: HashMap<String, EmbeddingMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingMetadata {
    pub signatory_id: String,
    pub label: String,
    pub embedding_model: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Mock embedding function (deterministic for testing)
/// In production: call OpenAI, Ollama, or local embedding model
fn mock_embedding(text: &str, seed: u64) -> Vec<f32> {
    // Generate deterministic embedding based on text hash
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    seed.hash(&mut hasher);
    let hash = hasher.finish();

    // Create 384-dim vector (typical embedding size)
    let mut vector = vec![0.0f32; 384];
    for i in 0..384 {
        let mixed = hash.wrapping_mul((i as u64).wrapping_add(7919));
        vector[i] = ((mixed as f32) / (u64::MAX as f32)) * 2.0 - 1.0; // Normalize to [-1, 1]
    }

    // Normalize to unit length
    let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        vector.iter_mut().for_each(|x| *x /= norm);
    }

    vector
}

/// Compute cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a > 0.0 && norm_b > 0.0 {
        dot_product / (norm_a * norm_b)
    } else {
        0.0
    }
}

/// Embedding generator: creates semantic vectors for signatories
pub struct EmbeddingGenerator {
    model: String,
    batch_size: usize,
}

impl EmbeddingGenerator {
    pub fn new(model: String, batch_size: usize) -> Self {
        Self { model, batch_size }
    }

    /// Generate embedding for a single signatory
    pub fn embed_signatory(&self, signatory: &Signatory) -> Embedding {
        let text = format!(
            "{} {} {}",
            signatory.label, signatory.snippet, signatory.source_uri
        );
        let vector = mock_embedding(&text, 42);

        Embedding {
            signatory_id: signatory.id.clone(),
            vector,
            computed_at: chrono::Utc::now(),
        }
    }

    /// Generate embeddings for multiple signatories (batched)
    pub async fn embed_batch(&self, signatories: Vec<Signatory>) -> Result<Vec<Embedding>> {
        let mut embeddings = Vec::new();

        for batch in signatories.chunks(self.batch_size) {
            let mut batch_tasks = Vec::new();

            for signatory in batch {
                let signatory_clone = signatory.clone();
                let model_clone = self.model.clone();

                batch_tasks.push(tokio::spawn(async move {
                    let gen = EmbeddingGenerator::new(model_clone, 64);
                    gen.embed_signatory(&signatory_clone)
                }));
            }

            for task in batch_tasks {
                if let Ok(embedding) = task.await {
                    embeddings.push(embedding);
                }
            }
        }

        Ok(embeddings)
    }

    /// Build searchable index from embeddings
    pub fn build_index(&self, embeddings: Vec<Embedding>) -> EmbeddingIndex {
        let mut index_embeddings = HashMap::new();
        let mut metadata = HashMap::new();

        for embedding in embeddings {
            index_embeddings.insert(embedding.signatory_id.clone(), embedding.vector);
            metadata.insert(
                embedding.signatory_id.clone(),
                EmbeddingMetadata {
                    signatory_id: embedding.signatory_id,
                    label: "".to_string(),
                    embedding_model: self.model.clone(),
                    created_at: embedding.computed_at,
                },
            );
        }

        EmbeddingIndex {
            embeddings: index_embeddings,
            metadata,
        }
    }
}

/// Vector search: find similar signatories using embeddings (zero-token traversal)
pub struct VectorSearch {
    index: EmbeddingIndex,
}

impl VectorSearch {
    pub fn new(index: EmbeddingIndex) -> Self {
        Self { index }
    }

    /// Find k most similar signatories to a query vector
    pub fn search(&self, query_vector: &[f32], k: usize) -> Vec<(String, f32)> {
        let mut similarities: Vec<(String, f32)> = self
            .index
            .embeddings
            .iter()
            .map(|(signatory_id, embedding)| {
                let sim = cosine_similarity(query_vector, embedding);
                (signatory_id.clone(), sim)
            })
            .collect();

        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        similarities.into_iter().take(k).collect()
    }

    /// Find k nearest neighbors to a signatory
    pub fn find_neighbors(&self, signatory_id: &str, k: usize) -> Vec<(String, f32)> {
        if let Some(query_vector) = self.index.embeddings.get(signatory_id) {
            self.search(query_vector, k + 1) // +1 because results include the query signatory itself
                .into_iter()
                .filter(|(id, _)| id != signatory_id)
                .take(k)
                .collect()
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.01);

        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &c).abs() < 0.01);
    }

    #[test]
    fn test_embedding_generation() {
        let signatory = Signatory::new(
            crate::types::SignatoryType::Function,
            "uri".to_string(),
            "testFunc".to_string(),
            "fn testFunc() {}".to_string(),
        );

        let gen = EmbeddingGenerator::new("mock".to_string(), 64);
        let embedding = gen.embed_signatory(&signatory);

        assert_eq!(embedding.vector.len(), 384);
        let norm: f32 = embedding.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_batch_embedding() {
        let signatories = vec![
            Signatory::new(
                crate::types::SignatoryType::Function,
                "uri1".to_string(),
                "func1".to_string(),
                "code1".to_string(),
            ),
            Signatory::new(
                crate::types::SignatoryType::Function,
                "uri2".to_string(),
                "func2".to_string(),
                "code2".to_string(),
            ),
        ];

        let gen = EmbeddingGenerator::new("mock".to_string(), 64);
        let embeddings = gen.embed_batch(signatories).await.unwrap();

        assert_eq!(embeddings.len(), 2);
    }

    #[test]
    fn test_vector_search() {
        let mut embeddings = Vec::new();
        for i in 0..5 {
            embeddings.push(Embedding {
                signatory_id: format!("signatory-{}", i),
                vector: mock_embedding(&format!("signatory content {}", i), i as u64),
                computed_at: chrono::Utc::now(),
            });
        }

        let gen = EmbeddingGenerator::new("mock".to_string(), 64);
        let index = gen.build_index(embeddings.clone());
        let search = VectorSearch::new(index);

        let query = embeddings[0].vector.clone();
        let results = search.search(&query, 3);

        assert_eq!(results.len(), 3);
        assert!(results[0].1 > 0.5);
    }
}
