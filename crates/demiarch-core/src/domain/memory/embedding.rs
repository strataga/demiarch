use crate::domain::memory::error::MemoryError;
use serde::{Deserialize, Serialize};

use super::MemoryLayer;

/// Trait for embedding providers
pub trait Embedder {
    fn embed(&self, model: &str, texts: &[&str]) -> Result<Embeddings, MemoryError>;
}

/// Embeddings storage for all memory layers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embeddings {
    pub index: Vec<f32>,
    pub timeline: Vec<f32>,
    pub full: Vec<f32>,
    pub model: String,
}

impl Embeddings {
    pub fn new(model: &str, index: Vec<f32>, timeline: Vec<f32>, full: Vec<f32>) -> Self {
        Self {
            index,
            timeline,
            full,
            model: model.to_string(),
        }
    }

    /// Get embedding for specified layer
    pub fn get(&self, layer: MemoryLayer) -> &[f32] {
        match layer {
            MemoryLayer::Index => &self.index,
            MemoryLayer::Timeline => &self.timeline,
            MemoryLayer::Full => &self.full,
        }
    }

    pub fn into_vec_for_layer(self, layer: MemoryLayer) -> Vec<f32> {
        match layer {
            MemoryLayer::Index => self.index,
            MemoryLayer::Timeline => self.timeline,
            MemoryLayer::Full => self.full,
        }
    }
}
