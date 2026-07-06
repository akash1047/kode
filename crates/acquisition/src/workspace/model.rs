use std::path::{Path, PathBuf};

/// A member package within a workspace.
///
/// Carries both the member's relative path and the path to its manifest,
/// which may differ (e.g., in Cargo workspaces the manifest is always
/// `Cargo.toml` within the member directory).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMember {
    pub(crate) relative_path: PathBuf,
    pub(crate) manifest_path: PathBuf,
}

impl WorkspaceMember {
    pub fn relative_path(&self) -> &Path {
        &self.relative_path
    }

    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }
}

/// Discriminated workspace structure kind.
///
/// # Invariants
///
/// - [`WorkspaceKind::None`] is the fallback when no detector matches.
/// - [`WorkspaceKind::SinglePackage`] has exactly one manifest.
/// - [`WorkspaceKind::CargoWorkspace`] has at least one member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceKind {
    CargoWorkspace {
        root_manifest: PathBuf,
        members: Vec<WorkspaceMember>,
    },
    SinglePackage {
        manifest: PathBuf,
    },
    None,
}

/// Detected workspace structure for a repository root.
///
/// Produced by [`WorkspaceDetector`](super::WorkspaceDetector) implementations
/// and consumed by [`RepositoryDiscovery`](crate::discovery::RepositoryDiscovery)
/// to determine manifest paths and project structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub(crate) kind: WorkspaceKind,
}

impl Workspace {
    pub fn none() -> Self {
        Self {
            kind: WorkspaceKind::None,
        }
    }

    pub fn kind(&self) -> &WorkspaceKind {
        &self.kind
    }

    pub fn manifest_paths(&self) -> Vec<&PathBuf> {
        match &self.kind {
            WorkspaceKind::CargoWorkspace {
                root_manifest,
                members,
            } => {
                let mut paths = vec![root_manifest];
                for member in members {
                    paths.push(&member.manifest_path);
                }
                paths
            }
            WorkspaceKind::SinglePackage { manifest } => vec![manifest],
            WorkspaceKind::None => vec![],
        }
    }
}
