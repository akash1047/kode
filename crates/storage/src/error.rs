use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("storage backend error: {0}")]
    Backend(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("revision not found: {0}")]
    RevisionNotFound(u64),

    #[error("repository not found: {0}")]
    RepositoryNotFound(String),

    #[error("incompatible schema version: stored {stored}, expected {expected}")]
    SchemaVersionMismatch { stored: String, expected: String },

    #[error("transaction failed: {0}")]
    TransactionFailed(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("database error: {0}")]
    Database(String),
}

impl From<rusqlite::Error> for StorageError {
    fn from(e: rusqlite::Error) -> Self {
        StorageError::Database(e.to_string())
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(e: serde_json::Error) -> Self {
        StorageError::Serialization(e.to_string())
    }
}

impl From<kode_graph::serialization::SerializationError> for StorageError {
    fn from(e: kode_graph::serialization::SerializationError) -> Self {
        StorageError::Serialization(e.to_string())
    }
}
