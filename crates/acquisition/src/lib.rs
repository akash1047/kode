//! Acquisition subsystem: discovers and parses repository facts.
//!
//! # Extension Points
//!
//! The crate supports extensible detection through three detector traits:
//!
//! - [`WorkspaceDetector`] — detects workspace structure (workspace, single package, or none)
//! - [`ManifestDetector`] — detects manifest files by filename
//! - [`LanguageDetector`] — detects programming language from file paths
//!
//! Each detector trait has a corresponding registry that manages multiple detectors:
//!
//! - [`WorkspaceRegistry`] — ordered collection of workspace detectors; returns first non-None match
//! - [`ManifestRegistry`] — ordered collection of manifest detectors; returns first match
//! - [`LanguageRegistry`] — ordered collection of language detectors; returns first match
//!
//! # Adding a new language
//!
//! 1. Add a variant to [`Language`].
//! 2. Implement [`LanguageDetector`] for your detector type.
//! 3. Register your detector in a [`LanguageRegistry`]:
//!
//! ```ignore
//! let mut registry = LanguageRegistry::new();
//! registry.register(Box::new(MyLanguageDetector));
//! ```
//!
//! # Adding a new workspace type
//!
//! 1. If needed, add a variant to [`WorkspaceKind`].
//! 2. Implement [`WorkspaceDetector`] for your detector.
//! 3. Register in a [`WorkspaceRegistry`].
//!
//! # Adding a new manifest type
//!
//! 1. Add a variant to [`ManifestKind`].
//! 2. Implement [`ManifestDetector`] for your detector.
//! 3. Register in a [`ManifestRegistry`].

#[cfg(test)]
use tempfile as _;

pub mod discovery;
pub mod error;
pub mod language;
pub mod manifest;
pub mod repository;
pub mod snapshot;
pub mod workspace;

mod walk;

pub use discovery::{discover, DefaultRepositoryDiscovery, RepositoryDiscovery};
pub use error::Error;
pub use language::{
    Language, LanguageDetector, LanguageRegistry, ExtensionLanguageDetector,
};
pub use manifest::{
    Manifest, ManifestKind, ManifestDetector, ManifestRegistry, CargoManifestDetector,
};
pub use repository::{Repository, RepositoryId, RepositoryIdentityService};
pub use snapshot::{
    DirectoryInventory, FileInventory, FileMetadata, LanguageInventory, ManifestInventory,
    RepositoryDirectory, RepositoryFile, RepositorySnapshot, SnapshotBuilder,
};
pub use workspace::{
    CargoWorkspaceDetector, Workspace, WorkspaceDetector, WorkspaceKind, WorkspaceMember,
    WorkspaceRegistry,
};
