use thiserror::Error;

/// Errors that can occur when working with progressive memory
#[derive(Debug, Error)]
pub enum MemoryError {
    /// Invalid input provided to the memory subsystem
    #[error("Invalid memory input: {0}")]
    InvalidInput(String),

    /// Errors produced while generating embeddings
    #[error(transparent)]
    Embedding(#[from] EmbeddingError),

    /// Errors persisting or loading memory data
    #[error("Storage error: {0}")]
    Storage(String),
}

/// Errors specific to embedding generation
#[derive(Debug, Error)]
pub enum EmbeddingError {
    #[error("Embedding failed for model {0}: {1}")]
    EmbeddingFailed(String, String),

    #[error("Dimension mismatch: expected {0}, got {1}")]
    DimensionMismatch(usize, usize),
}

impl MemoryError {
    pub fn invalid<T: Into<String>>(msg: T) -> Self {
        MemoryError::InvalidInput(msg.into())
    }

    pub fn storage<T: Into<String>>(msg: T) -> Self {
        MemoryError::Storage(msg.into())
    }
}
