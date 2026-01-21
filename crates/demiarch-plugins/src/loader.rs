//! Plugin loading and discovery

use crate::{
    PluginError, PluginManifest, PluginResult,
    license::{License, verify_license},
};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Load and parse a plugin manifest from disk (JSON)
pub fn load_manifest(path: &Path) -> PluginResult<PluginManifest> {
    let canonical_path = resolve_plugin_path(path)?;

    let metadata = fs::metadata(&canonical_path).map_err(PluginError::IoError)?;
    const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
    if metadata.len() > MAX_MANIFEST_BYTES {
        return Err(PluginError::ValidationFailed(
            "Manifest file too large".to_string(),
        ));
    }

    let data = fs::read_to_string(&canonical_path).map_err(PluginError::IoError)?;
    let manifest: PluginManifest = serde_json::from_str(&data)
        .map_err(|e| PluginError::ValidationFailed(format!("Invalid manifest JSON: {e}")))?;

    validate_manifest(&manifest)?;

    if license_enforcement_enabled() {
        let license_path = resolve_license_path(&canonical_path)?;
        let license: License = read_license(&license_path)?;
        verify_license(&manifest, &license)?;
    }

    Ok(manifest)
}

/// Load WASM module bytes with a size cap
pub fn load_wasm_bytes(path: &Path, max_bytes: usize) -> PluginResult<Vec<u8>> {
    let canonical_path = resolve_plugin_path(path)?;

    let metadata = fs::metadata(&canonical_path).map_err(PluginError::IoError)?;
    if metadata.len() as usize > max_bytes {
        return Err(PluginError::ValidationFailed(format!(
            "WASM module too large: {} bytes (limit {})",
            metadata.len(),
            max_bytes
        )));
    }

    fs::read(&canonical_path).map_err(PluginError::IoError)
}

fn validate_manifest(manifest: &PluginManifest) -> PluginResult<()> {
    if manifest.id.trim().is_empty()
        || manifest.name.trim().is_empty()
        || manifest.version.trim().is_empty()
        || manifest.description.trim().is_empty()
        || manifest.author.trim().is_empty()
    {
        return Err(PluginError::ValidationFailed(
            "Manifest fields cannot be empty".to_string(),
        ));
    }

    let mut seen = std::collections::HashSet::new();
    for permission in &manifest.permissions {
        if !seen.insert(permission) {
            return Err(PluginError::ValidationFailed(
                "Duplicate permissions are not allowed".to_string(),
            ));
        }
    }

    Ok(())
}

fn resolve_plugin_path(path: &Path) -> PluginResult<PathBuf> {
    let base_dir = plugin_base_dir()?;
    let canonical_target = path.canonicalize().map_err(PluginError::IoError)?;

    let metadata = fs::symlink_metadata(&canonical_target).map_err(PluginError::IoError)?;
    if metadata.file_type().is_symlink() {
        return Err(PluginError::ValidationFailed(
            "Plugin paths cannot point to symlinks".to_string(),
        ));
    }
    if !metadata.file_type().is_file() {
        return Err(PluginError::ValidationFailed(
            "Plugin path must be a regular file".to_string(),
        ));
    }

    if !canonical_target.starts_with(&base_dir) {
        return Err(PluginError::ValidationFailed(format!(
            "Plugin path {:?} must reside under {:?}",
            canonical_target, base_dir
        )));
    }

    Ok(canonical_target)
}

fn plugin_base_dir() -> PluginResult<PathBuf> {
    let base = if let Ok(path) = std::env::var("DEMIARCH_PLUGIN_DIR") {
        PathBuf::from(path)
    } else if let Some(home) = dirs::home_dir() {
        home.join(".demiarch").join("plugins")
    } else {
        return Err(PluginError::ValidationFailed(
            "Unable to resolve plugin directory".to_string(),
        ));
    };

    fs::create_dir_all(&base).map_err(PluginError::IoError)?;
    let canonical_base = base.canonicalize().unwrap_or(base);
    Ok(canonical_base)
}

fn license_enforcement_enabled() -> bool {
    match std::env::var("DEMIARCH_REQUIRE_LICENSE") {
        // Default: enforced
        Err(_) => true,
        Ok(val) if val == "0" || val.to_lowercase() == "false" => {
            // Only allow disabling when explicitly opted into unsafe mode
            std::env::var("DEMIARCH_UNSAFE_ALLOW_UNLICENSED")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false)
        }
        Ok(_) => true,
    }
}

fn resolve_license_path(manifest_path: &Path) -> PluginResult<PathBuf> {
    if let Ok(explicit) = std::env::var("DEMIARCH_PLUGIN_LICENSE") {
        let explicit_path = PathBuf::from(explicit);
        return resolve_plugin_path(&explicit_path);
    }

    let default_path = manifest_path
        .parent()
        .map(|p| p.join("license.json"))
        .ok_or_else(|| PluginError::ValidationFailed("Manifest path missing parent".to_string()))?;

    resolve_plugin_path(&default_path)
}

fn read_license(path: &Path) -> PluginResult<License> {
    let data = fs::read_to_string(path).map_err(PluginError::IoError)?;
    serde_json::from_str(&data)
        .map_err(|e| PluginError::ValidationFailed(format!("Invalid license JSON: {e}")))
}
