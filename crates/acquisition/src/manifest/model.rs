use std::path::{Path, PathBuf};

/// Known categories of build manifests.
///
/// Each variant corresponds to a build system that kode understands.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ManifestKind {
    /// A Cargo package or workspace manifest (`Cargo.toml`).
    CargoManifest,
}

/// Identified build manifest within a repository.
///
/// Manifests are detected by [`ManifestDetector`](super::ManifestDetector)
/// implementations and collected into [`ManifestInventory`] during discovery.
///
/// [`ManifestInventory`]: crate::ManifestInventory
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Manifest {
    pub(crate) relative_path: PathBuf,
    pub(crate) kind: ManifestKind,
}

impl Manifest {
    pub fn new(relative_path: PathBuf, kind: ManifestKind) -> Self {
        Self {
            relative_path,
            kind,
        }
    }

    pub fn relative_path(&self) -> &Path {
        &self.relative_path
    }

    pub fn kind(&self) -> &ManifestKind {
        &self.kind
    }
}
