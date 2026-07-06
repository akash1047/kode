use crate::error::Error;
use crate::language::LanguageRegistry;
use crate::manifest::{Manifest, ManifestDetector, ManifestRegistry};
use crate::repository::Repository;
use crate::snapshot::{
    DirectoryInventory, FileInventory, LanguageInventory, ManifestInventory, RepositorySnapshot,
    SnapshotBuilder,
};
use crate::walk::walk_repository;
use crate::workspace::{WorkspaceDetector, WorkspaceRegistry};

#[derive(Default)]
pub struct RepositoryDiscovery {
    workspace_registry: WorkspaceRegistry,
    manifest_registry: ManifestRegistry,
    language_registry: LanguageRegistry,
}

impl RepositoryDiscovery {
    pub fn new(
        workspace_registry: WorkspaceRegistry,
        manifest_registry: ManifestRegistry,
        language_registry: LanguageRegistry,
    ) -> Self {
        Self {
            workspace_registry,
            manifest_registry,
            language_registry,
        }
    }

    pub fn run(&self, repository: &Repository) -> Result<RepositorySnapshot, Error> {
        let workspace = self.workspace_registry.detect(repository.root())?;

        let (files, directories) =
            walk_repository(repository.root(), &self.language_registry)?;

        let mut manifest_inventory = ManifestInventory::new();
        for manifest_rel_path in workspace.manifest_paths() {
            if let Some(filename) = manifest_rel_path.file_name().and_then(|n| n.to_str()) {
                if let Some(kind) = self.manifest_registry.detect(filename) {
                    manifest_inventory.push(Manifest::new(
                        manifest_rel_path.to_path_buf(),
                        kind,
                    ));
                }
            }
        }

        for file in &files {
            if let Some(filename) = file.relative_path().file_name().and_then(|n| n.to_str()) {
                if let Some(kind) = self.manifest_registry.detect(filename) {
                    if !manifest_inventory.contains_path(file.relative_path()) {
                        manifest_inventory.push(Manifest::new(
                            file.relative_path().to_path_buf(),
                            kind,
                        ));
                    }
                }
            }
        }
        manifest_inventory.sort();

        let language_inventory = LanguageInventory::from_files(&files);
        let file_inventory = FileInventory::new(files);
        let directory_inventory = DirectoryInventory::new(directories);

        SnapshotBuilder::new()
            .repository(repository.clone())
            .workspace(workspace)
            .file_inventory(file_inventory)
            .directory_inventory(directory_inventory)
            .manifest_inventory(manifest_inventory)
            .language_inventory(language_inventory)
            .build()
            .map_err(Error::WorkspaceMemberResolution)
    }
}

pub type DefaultRepositoryDiscovery = RepositoryDiscovery;

pub fn discover(repository: &Repository) -> Result<RepositorySnapshot, Error> {
    RepositoryDiscovery::default().run(repository)
}
