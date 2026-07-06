use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("repository path does not exist: {0}")]
    NotFound(PathBuf),

    #[error("repository path is not a directory: {0}")]
    NotADirectory(PathBuf),

    #[error("failed to canonicalize path `{path}`: {source}")]
    Canonicalization {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to traverse directory `{path}`: {source}")]
    Traversal {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to read manifest `{path}`: {source}")]
    ManifestRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to parse manifest `{path}`: {source}")]
    ManifestParse {
        path: PathBuf,
        source: toml::de::Error,
    },

    #[error("workspace member resolution failed: {0}")]
    WorkspaceMemberResolution(String),
}
