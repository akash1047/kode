use std::path::{Path, PathBuf};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub(crate) kind: WorkspaceKind,
}

impl Workspace {
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
