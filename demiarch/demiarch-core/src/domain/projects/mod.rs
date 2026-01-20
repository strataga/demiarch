//! Project domain module
//!
//! Contains project entities, value objects, and repository traits
//! related to project management.

pub mod entity;
pub mod repository;
pub mod service;
pub mod value_object;

// Re-export project types
pub use entity::*;
pub use repository::*;
pub use service::*;
pub use value_object::*;
