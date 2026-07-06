use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ManifestKind {
    CargoManifest,
}

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
