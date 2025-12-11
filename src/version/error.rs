use thiserror::Error;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Failed to connect to database: {0}")]
    Connection(#[from] sqlx::Error),

    #[error("Failed to create schema: {0}")]
    SchemaCreation(String),

    #[error("Database query failed: {0}")]
    Query(String),
}
