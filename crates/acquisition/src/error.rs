use std::path::PathBuf;
use thiserror::Error;

/// Acquisition subsystem errors.
///
/// # Recovery Strategy
///
/// - [`NotFound`](Error::NotFound) and [`NotADirectory`](Error::NotADirectory):
///   caller should validate the path before retrying.
/// - [`Traversal`](Error::Traversal): filesystem-level failure; caller may
///   retry or abort depending on severity.
/// - [`ManifestRead`](Error::ManifestRead) and [`ManifestParse`](Error::ManifestParse):
///   individual manifest failures are non-fatal; other manifests are unaffected.
/// - [`WorkspaceMemberResolution`](Error::WorkspaceMemberResolution):
///   indicates a builder invariant was violated (programming error).
///
/// # Ownership
///
/// These errors are owned by the acquisition crate and should not be
/// re-exported across subsystem boundaries. Downstream crates map
/// acquisition errors into their own error types.
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
