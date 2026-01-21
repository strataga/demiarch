//! Knowledge graph infrastructure implementations
//!
//! This module contains concrete implementations of the knowledge graph
//! repository trait using SQLite.

mod repository;

pub use repository::SqliteKnowledgeGraphRepository;
