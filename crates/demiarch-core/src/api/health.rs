//! Health API
//!
//! Provides system health checks and diagnostics for GUI.

use crate::Result;
use serde::{Deserialize, Serialize};

use super::get_database;

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    pub message: Option<String>,
}

/// Health status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Ok,
    Warning,
    Error,
}

/// Overall system health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub overall_status: HealthStatus,
    pub checks: Vec<HealthCheck>,
    pub timestamp: String,
}

/// Run all health checks (doctor command)
pub async fn doctor() -> Result<HealthReport> {
    let mut checks = Vec::new();
    let mut worst_status = HealthStatus::Ok;

    // Check database connection
    let db_check = check_database().await;
    if db_check.status == HealthStatus::Error {
        worst_status = HealthStatus::Error;
    } else if db_check.status == HealthStatus::Warning && worst_status != HealthStatus::Error {
        worst_status = HealthStatus::Warning;
    }
    checks.push(db_check);

    // Check config file
    let config_check = check_config().await;
    if config_check.status == HealthStatus::Error && worst_status != HealthStatus::Error {
        worst_status = HealthStatus::Error;
    } else if config_check.status == HealthStatus::Warning && worst_status == HealthStatus::Ok {
        worst_status = HealthStatus::Warning;
    }
    checks.push(config_check);

    // Check data directory
    let data_check = check_data_directory().await;
    if data_check.status == HealthStatus::Error && worst_status != HealthStatus::Error {
        worst_status = HealthStatus::Error;
    } else if data_check.status == HealthStatus::Warning && worst_status == HealthStatus::Ok {
        worst_status = HealthStatus::Warning;
    }
    checks.push(data_check);

    Ok(HealthReport {
        overall_status: worst_status,
        checks,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// Check database connection
async fn check_database() -> HealthCheck {
    match get_database().await {
        Ok(db) => {
            // Try a simple query
            match sqlx::query("SELECT 1").fetch_one(db.pool()).await {
                Ok(_) => HealthCheck {
                    name: "Database".to_string(),
                    status: HealthStatus::Ok,
                    message: Some("Connected and responsive".to_string()),
                },
                Err(e) => HealthCheck {
                    name: "Database".to_string(),
                    status: HealthStatus::Error,
                    message: Some(format!("Query failed: {}", e)),
                },
            }
        }
        Err(e) => HealthCheck {
            name: "Database".to_string(),
            status: HealthStatus::Error,
            message: Some(format!("Connection failed: {}", e)),
        },
    }
}

/// Check config file
async fn check_config() -> HealthCheck {
    let config_path = dirs::config_dir().map(|p| p.join("demiarch").join("config.toml"));

    match config_path {
        Some(path) if path.exists() => HealthCheck {
            name: "Configuration".to_string(),
            status: HealthStatus::Ok,
            message: Some(format!("Found at {}", path.display())),
        },
        Some(path) => HealthCheck {
            name: "Configuration".to_string(),
            status: HealthStatus::Warning,
            message: Some(format!("Not found at {} (using defaults)", path.display())),
        },
        None => HealthCheck {
            name: "Configuration".to_string(),
            status: HealthStatus::Warning,
            message: Some("Could not determine config directory".to_string()),
        },
    }
}

/// Check data directory
async fn check_data_directory() -> HealthCheck {
    let data_path = dirs::data_dir().map(|p| p.join("demiarch"));

    match data_path {
        Some(path) => {
            if path.exists() {
                // Check if we can write to it
                let test_file = path.join(".health_check");
                match tokio::fs::write(&test_file, "test").await {
                    Ok(_) => {
                        let _ = tokio::fs::remove_file(&test_file).await;
                        HealthCheck {
                            name: "Data Directory".to_string(),
                            status: HealthStatus::Ok,
                            message: Some(format!("Writable at {}", path.display())),
                        }
                    }
                    Err(e) => HealthCheck {
                        name: "Data Directory".to_string(),
                        status: HealthStatus::Error,
                        message: Some(format!("Not writable: {}", e)),
                    },
                }
            } else {
                HealthCheck {
                    name: "Data Directory".to_string(),
                    status: HealthStatus::Warning,
                    message: Some(format!(
                        "Does not exist at {} (will be created)",
                        path.display()
                    )),
                }
            }
        }
        None => HealthCheck {
            name: "Data Directory".to_string(),
            status: HealthStatus::Error,
            message: Some("Could not determine data directory".to_string()),
        },
    }
}

/// System information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub version: String,
    pub database_path: Option<String>,
    pub config_path: Option<String>,
    pub data_path: Option<String>,
}

/// Get system information
pub fn get_system_info() -> SystemInfo {
    SystemInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        database_path: dirs::data_dir()
            .map(|p| p.join("demiarch").join("demiarch.db").display().to_string()),
        config_path: dirs::config_dir()
            .map(|p| p.join("demiarch").join("config.toml").display().to_string()),
        data_path: dirs::data_dir().map(|p| p.join("demiarch").display().to_string()),
    }
}
