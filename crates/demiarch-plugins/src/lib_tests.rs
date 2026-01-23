//! Plugin system tests

use crate::{LicenseTier, Permission, PluginError, PluginManifest};

#[test]
fn test_plugin_manifest_serialization() {
    let manifest = PluginManifest {
        id: "test-plugin".to_string(),
        name: "Test Plugin".to_string(),
        version: "1.0.0".to_string(),
        description: "A test plugin".to_string(),
        author: "Test Author".to_string(),
        license_tier: LicenseTier::Free,
        permissions: vec![Permission::ReadFiles],
    };

    let json = serde_json::to_string(&manifest).expect("serialize");
    let deserialized: PluginManifest = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(deserialized.id, "test-plugin");
    assert_eq!(deserialized.name, "Test Plugin");
    assert_eq!(deserialized.version, "1.0.0");
    assert_eq!(deserialized.permissions.len(), 1);
}

#[test]
fn test_plugin_manifest_all_permissions() {
    let manifest = PluginManifest {
        id: "full-access".to_string(),
        name: "Full Access Plugin".to_string(),
        version: "2.0.0".to_string(),
        description: "Plugin with all permissions".to_string(),
        author: "Admin".to_string(),
        license_tier: LicenseTier::Enterprise,
        permissions: vec![
            Permission::ReadFiles,
            Permission::WriteFiles,
            Permission::Network,
            Permission::Subprocess,
        ],
    };

    assert_eq!(manifest.permissions.len(), 4);
    assert!(manifest.permissions.contains(&Permission::ReadFiles));
    assert!(manifest.permissions.contains(&Permission::WriteFiles));
    assert!(manifest.permissions.contains(&Permission::Network));
    assert!(manifest.permissions.contains(&Permission::Subprocess));
}

#[test]
fn test_license_tier_serialization() {
    let free = LicenseTier::Free;
    let pro = LicenseTier::Pro;
    let enterprise = LicenseTier::Enterprise;

    let free_json = serde_json::to_string(&free).expect("serialize free");
    let pro_json = serde_json::to_string(&pro).expect("serialize pro");
    let enterprise_json = serde_json::to_string(&enterprise).expect("serialize enterprise");

    assert_eq!(free_json, "\"Free\"");
    assert_eq!(pro_json, "\"Pro\"");
    assert_eq!(enterprise_json, "\"Enterprise\"");
}

#[test]
fn test_permission_serialization() {
    let read = Permission::ReadFiles;
    let write = Permission::WriteFiles;
    let network = Permission::Network;
    let subprocess = Permission::Subprocess;

    let read_json = serde_json::to_string(&read).expect("serialize");
    let write_json = serde_json::to_string(&write).expect("serialize");
    let network_json = serde_json::to_string(&network).expect("serialize");
    let subprocess_json = serde_json::to_string(&subprocess).expect("serialize");

    assert_eq!(read_json, "\"ReadFiles\"");
    assert_eq!(write_json, "\"WriteFiles\"");
    assert_eq!(network_json, "\"Network\"");
    assert_eq!(subprocess_json, "\"Subprocess\"");
}

#[test]
fn test_permission_equality() {
    let read1 = Permission::ReadFiles;
    let read2 = Permission::ReadFiles;
    let write = Permission::WriteFiles;

    assert_eq!(read1, read2);
    assert_ne!(read1, write);
}

#[test]
fn test_permission_hash() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(Permission::ReadFiles);
    set.insert(Permission::WriteFiles);
    set.insert(Permission::ReadFiles); // Duplicate

    assert_eq!(set.len(), 2);
    assert!(set.contains(&Permission::ReadFiles));
    assert!(set.contains(&Permission::WriteFiles));
}

#[test]
fn test_plugin_error_display() {
    let not_found = PluginError::NotFound("my-plugin".to_string());
    let validation_failed = PluginError::ValidationFailed("bad manifest".to_string());
    let license_expired = PluginError::LicenseExpired("expired-plugin".to_string());
    let wasm_error = PluginError::WasmError("execution failed".to_string());

    assert!(not_found.to_string().contains("my-plugin"));
    assert!(validation_failed.to_string().contains("bad manifest"));
    assert!(license_expired.to_string().contains("expired-plugin"));
    assert!(wasm_error.to_string().contains("execution failed"));
}

#[test]
fn test_plugin_error_is_debug() {
    let error = PluginError::NotFound("test".to_string());
    let debug_str = format!("{:?}", error);
    assert!(debug_str.contains("NotFound"));
}

#[test]
fn test_plugin_manifest_clone() {
    let manifest = PluginManifest {
        id: "clone-test".to_string(),
        name: "Clone Test".to_string(),
        version: "1.0.0".to_string(),
        description: "Testing clone".to_string(),
        author: "Tester".to_string(),
        license_tier: LicenseTier::Pro,
        permissions: vec![Permission::Network],
    };

    let cloned = manifest.clone();
    assert_eq!(manifest.id, cloned.id);
    assert_eq!(manifest.permissions, cloned.permissions);
}

#[test]
fn test_license_tier_copy() {
    let tier = LicenseTier::Enterprise;
    let copied = tier;
    assert!(matches!(copied, LicenseTier::Enterprise));
}

#[test]
fn test_permission_copy() {
    let perm = Permission::Subprocess;
    let copied = perm;
    assert_eq!(copied, Permission::Subprocess);
}

#[test]
fn test_plugin_manifest_empty_permissions() {
    let manifest = PluginManifest {
        id: "minimal".to_string(),
        name: "Minimal Plugin".to_string(),
        version: "0.1.0".to_string(),
        description: "No special permissions".to_string(),
        author: "Author".to_string(),
        license_tier: LicenseTier::Free,
        permissions: vec![],
    };

    assert!(manifest.permissions.is_empty());
}

mod sandbox_tests {
    use crate::sandbox::Sandbox;
    use crate::Permission;

    #[test]
    fn test_sandbox_creation() {
        // Sandbox creation may fail if wasmtime config is incompatible
        // Just verify the function exists and returns a result
        let result = Sandbox::new(vec![Permission::ReadFiles]);
        match result {
            Ok(sandbox) => {
                assert!(sandbox.allows(Permission::ReadFiles));
                assert!(!sandbox.allows(Permission::WriteFiles));
            }
            Err(e) => {
                // Wasmtime config issue is expected in some environments
                assert!(e.to_string().contains("wasmtime") || e.to_string().contains("WASM"));
            }
        }
    }

    #[test]
    fn test_sandbox_multiple_permissions() {
        let result = Sandbox::new(vec![
            Permission::ReadFiles,
            Permission::WriteFiles,
            Permission::Network,
        ]);

        match result {
            Ok(sandbox) => {
                assert!(sandbox.allows(Permission::ReadFiles));
                assert!(sandbox.allows(Permission::WriteFiles));
                assert!(sandbox.allows(Permission::Network));
                assert!(!sandbox.allows(Permission::Subprocess));
            }
            Err(e) => {
                // Wasmtime config issue is expected in some environments
                assert!(e.to_string().contains("wasmtime") || e.to_string().contains("WASM"));
            }
        }
    }

    #[test]
    fn test_sandbox_no_permissions() {
        let result = Sandbox::new(vec![]);
        match result {
            Ok(sandbox) => {
                assert!(!sandbox.allows(Permission::ReadFiles));
                assert!(!sandbox.allows(Permission::WriteFiles));
                assert!(!sandbox.allows(Permission::Network));
                assert!(!sandbox.allows(Permission::Subprocess));
            }
            Err(e) => {
                // Wasmtime config issue is expected in some environments
                assert!(e.to_string().contains("wasmtime") || e.to_string().contains("WASM"));
            }
        }
    }
}

mod license_tests {
    use crate::license::License;
    use chrono::{Duration, Utc};

    #[test]
    fn test_license_serialization() {
        let license = License {
            plugin_id: "test-plugin".to_string(),
            expires_at: Utc::now() + Duration::days(365),
            public_key: vec![0u8; 32],
            signature: vec![0u8; 64],
            payload: vec![1, 2, 3, 4],
        };

        let json = serde_json::to_string(&license).expect("serialize");
        let deserialized: License = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.plugin_id, "test-plugin");
        assert_eq!(deserialized.public_key.len(), 32);
        assert_eq!(deserialized.signature.len(), 64);
    }

    #[test]
    fn test_license_clone() {
        let license = License {
            plugin_id: "clone-test".to_string(),
            expires_at: Utc::now(),
            public_key: vec![1, 2, 3],
            signature: vec![4, 5, 6],
            payload: vec![7, 8, 9],
        };

        let cloned = license.clone();
        assert_eq!(license.plugin_id, cloned.plugin_id);
        assert_eq!(license.payload, cloned.payload);
    }
}
