//! Database migration system using sqlx

use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use tracing::info;

/// Migration definition
pub struct Migration {
    /// Migration version (e.g., "001", "002")
    pub version: &'static str,
    /// Migration description
    pub description: &'static str,
    /// SQL to apply the migration (UP)
    pub up: &'static str,
    /// SQL to revert the migration (DOWN)
    pub down: &'static str,
}

/// Migration registry
pub struct Migrations {
    migrations: HashMap<&'static str, Migration>,
}

impl Migrations {
    /// Create new migrations registry
    pub fn new() -> Self {
        let mut migrations = HashMap::new();

        // Add initial schema migration
        migrations.insert(
            "001",
            Migration {
                version: "001",
                description: "Initial schema creation",
                up: crate::infrastructure::db::schema::create_tables_sql(),
                down: crate::infrastructure::db::schema::drop_tables_sql(),
            },
        );

        Self { migrations }
    }

    /// Get all migrations sorted by version
    pub fn get_all(&self) -> Vec<&Migration> {
        let mut migrations: Vec<_> = self.migrations.values().collect();
        migrations.sort_by(|a, b| a.version.cmp(b.version));
        migrations
    }

    /// Get migration by version
    pub fn get(&self, version: &str) -> Option<&Migration> {
        self.migrations.get(version)
    }
}

/// Migration manager
pub struct MigrationManager {
    migrations: Migrations,
}

impl MigrationManager {
    /// Create new migration manager
    pub fn new() -> Self {
        Self {
            migrations: Migrations::new(),
        }
    }

    /// Get current schema version from database
    pub async fn get_current_version(&self, pool: &SqlitePool) -> anyhow::Result<Option<String>> {
        let result = sqlx::query(crate::infrastructure::db::schema::get_schema_version_sql())
            .fetch_optional(pool)
            .await?;

        match result {
            Some(row) => {
                let version: String = row.get("version");
                Ok(Some(version))
            }
            None => Ok(None),
        }
    }

    /// Get pending migrations
    pub async fn get_pending_migrations(
        &self,
        pool: &SqlitePool,
    ) -> anyhow::Result<Vec<&Migration>> {
        let current_version = self.get_current_version(pool).await?;
        let all_migrations = self.migrations.get_all();

        if current_version.is_none() {
            return Ok(all_migrations);
        }

        let current_version = current_version.unwrap();
        let pending_migrations: Vec<&Migration> = all_migrations
            .into_iter()
            .filter(|m| m.version > current_version.as_str())
            .collect();

        Ok(pending_migrations)
    }

    /// Run pending migrations
    pub async fn run_migrations(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        // First, ensure schema_version table exists
        self.ensure_schema_version_table(pool).await?;

        // Get pending migrations
        let pending_migrations = self.get_pending_migrations(pool).await?;

        if pending_migrations.is_empty() {
            info!("Database is up to date");
            return Ok(());
        }

        info!("Running {} pending migrations", pending_migrations.len());

        for migration in pending_migrations {
            info!(
                "Applying migration {}: {}",
                migration.version, migration.description
            );

            let mut tx = pool.begin().await?;

            // Apply migration
            sqlx::raw_sql(migration.up).execute(&mut *tx).await?;

            // Record migration
            let insert_sql = crate::infrastructure::db::schema::insert_schema_version_sql(
                migration.version,
                migration.description,
            );

            sqlx::raw_sql(&insert_sql).execute(&mut *tx).await?;

            tx.commit().await?;

            info!("Applied migration {}", migration.version);
        }

        Ok(())
    }

    /// Rollback migration
    pub async fn rollback_migration(&self, pool: &SqlitePool, version: &str) -> anyhow::Result<()> {
        let migration = self
            .migrations
            .get(version)
            .ok_or_else(|| anyhow::anyhow!("Migration {} not found", version))?;

        info!(
            "Rolling back migration {}: {}",
            migration.version, migration.description
        );

        let mut tx = pool.begin().await?;

        // Rollback migration
        sqlx::raw_sql(migration.down).execute(&mut *tx).await?;

        // Remove migration record
        sqlx::query("DELETE FROM schema_version WHERE version = $1")
            .bind(version)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        info!("Rolled back migration {}", migration.version);

        Ok(())
    }

    /// Ensure schema_version table exists
    async fn ensure_schema_version_table(&self, pool: &SqlitePool) -> anyhow::Result<()> {
        let create_table_sql = crate::infrastructure::db::schema::create_schema_version_table_sql();

        sqlx::raw_sql(create_table_sql).execute(pool).await?;

        Ok(())
    }
}

/// Run all pending migrations
pub async fn run_migrations(pool: &SqlitePool) -> anyhow::Result<()> {
    let manager = MigrationManager::new();
    manager.run_migrations(pool).await
}

/// Get current schema version
pub async fn get_schema_version(pool: &SqlitePool) -> anyhow::Result<Option<String>> {
    let manager = MigrationManager::new();
    manager.get_current_version(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_struct() {
        let migration = Migration {
            version: "001",
            description: "Initial schema",
            up: "CREATE TABLE test (id INTEGER);",
            down: "DROP TABLE test;",
        };

        assert_eq!(migration.version, "001");
        assert_eq!(migration.description, "Initial schema");
        assert!(migration.up.contains("CREATE TABLE"));
        assert!(migration.down.contains("DROP TABLE"));
    }
}
