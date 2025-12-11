use std::path::Path;

use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use tracing::{debug, info};

use crate::version::error::CacheError;

pub struct Cache {
    pool: SqlitePool,
    #[allow(dead_code)]
    refresh_interval: i64,
}

impl Cache {
    pub async fn new(db_path: &Path, refresh_interval: i64) -> Result<Self, CacheError> {
        info!("Initializing cache database at {:?}", db_path);

        let options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options).await?;
        debug!("Database connection established");

        let cache = Self {
            pool,
            refresh_interval,
        };

        cache.create_schema().await?;
        info!("Cache initialized successfully");

        Ok(cache)
    }

    async fn create_schema(&self) -> Result<(), CacheError> {
        debug!("Creating database schema");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS packages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                registry_type TEXT NOT NULL,
                package_name TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                UNIQUE(registry_type, package_name)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| CacheError::SchemaCreation(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_updated_at ON packages(updated_at)
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| CacheError::SchemaCreation(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_registry_package ON packages(registry_type, package_name)
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| CacheError::SchemaCreation(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                package_id INTEGER NOT NULL,
                version TEXT NOT NULL,
                FOREIGN KEY (package_id) REFERENCES packages(id) ON DELETE CASCADE,
                UNIQUE(package_id, version)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| CacheError::SchemaCreation(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_package_id ON versions(package_id)
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| CacheError::SchemaCreation(e.to_string()))?;

        debug!("Database schema created successfully");
        Ok(())
    }

    pub async fn table_exists(&self, table_name: &str) -> Result<bool, CacheError> {
        let result: (i32,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM sqlite_master
            WHERE type='table' AND name=?
            "#,
        )
        .bind(table_name)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| CacheError::Query(e.to_string()))?;

        Ok(result.0 > 0)
    }
}
