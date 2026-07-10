//! Storage error type.

use quill_core::CoreError;

/// Errors raised by the persistence layer.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// A database/SQL error from SQLx.
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// A schema migration failed to apply.
    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    /// JSON (de)serialization of a stored blob failed.
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// A domain rule from `quill-core` was violated.
    #[error(transparent)]
    Core(#[from] CoreError),

    /// A referenced entity was not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// A filesystem path was invalid for a vault_db.
    #[error("invalid vault path: {0}")]
    InvalidVaultPath(String),
}

/// Storage result alias.
pub type Result<T, E = StorageError> = std::result::Result<T, E>;
