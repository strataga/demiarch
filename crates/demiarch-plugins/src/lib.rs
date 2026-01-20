//! Demiarch Plugin System
//!
//! Provides WASM-based sandboxed plugin execution with:
//! - Plugin loading and validation
//! - WASM sandboxing via wasmtime
//! - Offline license verification (ed25519 signatures)
//! - Plugin marketplace integration

pub mod license;
pub mod loader;
pub mod registry;
pub mod sandbox;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Plugin validation failed: {0}")]
    ValidationFailed(String),

    #[error("License expired for plugin '{0}'")]
    LicenseExpired(String),

    #[error("WASM execution error: {0}")]
    WasmError(String),

    #[error("Plugin IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type PluginResult<T> = Result<T, PluginError>;

/// Plugin manifest
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license_tier: LicenseTier,
    pub permissions: Vec<Permission>,
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub enum LicenseTier {
    Free,
    Pro,
    Enterprise,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum Permission {
    ReadFiles,
    WriteFiles,
    Network,
    Subprocess,
}
