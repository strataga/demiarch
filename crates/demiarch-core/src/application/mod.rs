//! Application service layer
//!
//! This layer orchestrates domain operations, handles transactions,
//! and provides the public API for the core functionality.

pub mod errors;
pub mod services;
pub mod validators;

pub use errors::ApplicationError;
