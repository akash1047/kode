use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::language::Language;
use crate::manifest::Manifest;
use crate::repository::Repository;
use crate::workspace::Workspace;

/// Filesystem metadata for a discovered file.
///
/// Captured during repository walking for incremental change detection.
#[derive(Debug, Clone)]
pub struct FileMetadata {
    size: u64,
    modified: Option<SystemTime>,
}

impl FileMetadata {
    pub fn new(size: u64, modified: Option<SystemTime>) -> Self {
        Self { size, modified }
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn modified(&self) -> Option<&SystemTime> {
        self.modified.as_ref()
    }
}

/// A single file discovered during repository traversal.
///
/// Carries its relative path, filesystem metadata, and detected language.
/// Files are the primary unit of processing in subsequent pipeline stages.
#[derive(Debug, Clone)]
pub struct RepositoryFile {
    pub(crate) relative_path: PathBuf,
    pub(crate) metadata: FileMetadata,
    pub(crate) language: Option<Language>,
}

impl RepositoryFile {
    pub fn new(
        relative_path: impl Into<PathBuf>,
        metadata: FileMetadata,
        language: Option<Language>,
    ) -> Self {
        Self {
            relative_path: relative_path.into(),
            metadata,
            language,
        }
    }

    pub fn relative_path(&self) -> &Path {
        &self.relative_path
    }

    pub fn metadata(&self) -> &FileMetadata {
        &self.metadata
    }

    pub fn language(&self) -> Option<&Language> {
        self.language.as_ref()
    }

    pub fn size(&self) -> u64 {
        self.metadata.size
    }

    pub fn modified(&self) -> Option<&SystemTime> {
        self.metadata.modified()
    }
}

/// A directory discovered during repository traversal.
#[derive(Debug, Clone)]
pub struct RepositoryDirectory {
    pub(crate) relative_path: PathBuf,
}

impl RepositoryDirectory {
    pub fn new(relative_path: impl Into<PathBuf>) -> Self {
        Self {
            relative_path: relative_path.into(),
        }
    }

    pub fn relative_path(&self) -> &Path {
        &self.relative_path
    }
}

/// Ordered collection of discovered repository files.
///
/// Maintains insertion order and provides standard collection traits
/// for ergonomic construction from iterators.
///
/// # Invariants
///
/// - Files are sorted by relative path before being stored in [`RepositorySnapshot`].
/// - Every file is guaranteed to have a non-empty relative path.
#[derive(Debug, Clone, Default)]
pub struct FileInventory(Vec<RepositoryFile>);

impl FileInventory {
    pub fn new(files: Vec<RepositoryFile>) -> Self {
        Self(files)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, RepositoryFile> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn into_inner(self) -> Vec<RepositoryFile> {
        self.0
    }

    pub fn as_slice(&self) -> &[RepositoryFile] {
        &self.0
    }
}

impl IntoIterator for FileInventory {
    type Item = RepositoryFile;
    type IntoIter = std::vec::IntoIter<RepositoryFile>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a FileInventory {
    type Item = &'a RepositoryFile;
    type IntoIter = std::slice::Iter<'a, RepositoryFile>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl FromIterator<RepositoryFile> for FileInventory {
    fn from_iter<I: IntoIterator<Item = RepositoryFile>>(iter: I) -> Self {
        Self(Vec::from_iter(iter))
    }
}

impl Extend<RepositoryFile> for FileInventory {
    fn extend<I: IntoIterator<Item = RepositoryFile>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl AsRef<[RepositoryFile]> for FileInventory {
    fn as_ref(&self) -> &[RepositoryFile] {
        &self.0
    }
}

/// Ordered collection of discovered repository directories.
///
/// Mirrors [`FileInventory`] in structure and guarantees.
#[derive(Debug, Clone, Default)]
pub struct DirectoryInventory(Vec<RepositoryDirectory>);

impl DirectoryInventory {
    pub fn new(dirs: Vec<RepositoryDirectory>) -> Self {
        Self(dirs)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, RepositoryDirectory> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn into_inner(self) -> Vec<RepositoryDirectory> {
        self.0
    }

    pub fn as_slice(&self) -> &[RepositoryDirectory] {
        &self.0
    }
}

impl IntoIterator for DirectoryInventory {
    type Item = RepositoryDirectory;
    type IntoIter = std::vec::IntoIter<RepositoryDirectory>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a DirectoryInventory {
    type Item = &'a RepositoryDirectory;
    type IntoIter = std::slice::Iter<'a, RepositoryDirectory>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl FromIterator<RepositoryDirectory> for DirectoryInventory {
    fn from_iter<I: IntoIterator<Item = RepositoryDirectory>>(iter: I) -> Self {
        Self(Vec::from_iter(iter))
    }
}

impl Extend<RepositoryDirectory> for DirectoryInventory {
    fn extend<I: IntoIterator<Item = RepositoryDirectory>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl AsRef<[RepositoryDirectory]> for DirectoryInventory {
    fn as_ref(&self) -> &[RepositoryDirectory] {
        &self.0
    }
}

/// Ordered collection of detected build manifests.
///
/// Provides deduplication via [`contains_path`](Self::contains_path) and
/// deterministic ordering via [`sort`](Self::sort).
#[derive(Debug, Clone)]
pub struct ManifestInventory(Vec<Manifest>);

impl Default for ManifestInventory {
    fn default() -> Self {
        Self::new()
    }
}

impl ManifestInventory {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn from_manifests(manifests: Vec<Manifest>) -> Self {
        Self(manifests)
    }

    pub fn push(&mut self, m: Manifest) {
        self.0.push(m);
    }

    pub fn sort(&mut self) {
        self.0
            .sort_by(|a, b| a.relative_path().cmp(b.relative_path()));
    }

    pub fn contains_path(&self, path: &Path) -> bool {
        self.0.iter().any(|m| m.relative_path() == path)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Manifest> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn as_slice(&self) -> &[Manifest] {
        &self.0
    }

    pub fn paths(&self) -> impl Iterator<Item = &PathBuf> {
        self.0.iter().map(|m| &m.relative_path)
    }
}

impl IntoIterator for ManifestInventory {
    type Item = Manifest;
    type IntoIter = std::vec::IntoIter<Manifest>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a ManifestInventory {
    type Item = &'a Manifest;
    type IntoIter = std::slice::Iter<'a, Manifest>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl FromIterator<Manifest> for ManifestInventory {
    fn from_iter<I: IntoIterator<Item = Manifest>>(iter: I) -> Self {
        Self(Vec::from_iter(iter))
    }
}

impl Extend<Manifest> for ManifestInventory {
    fn extend<I: IntoIterator<Item = Manifest>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl AsRef<[Manifest]> for ManifestInventory {
    fn as_ref(&self) -> &[Manifest] {
        &self.0
    }
}

/// Set of languages present across discovered files.
///
/// Derived from [`RepositoryFile`] language annotations. Uses [`BTreeSet`]
/// for deterministic ordering.
#[derive(Debug, Clone)]
pub struct LanguageInventory(BTreeSet<Language>);

impl Default for LanguageInventory {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageInventory {
    pub fn new() -> Self {
        Self(BTreeSet::new())
    }

    pub fn from_languages(languages: BTreeSet<Language>) -> Self {
        Self(languages)
    }

    pub fn from_files(files: &[RepositoryFile]) -> Self {
        Self(files.iter().filter_map(|f| f.language.clone()).collect())
    }

    pub fn contains(&self, lang: &Language) -> bool {
        self.0.contains(lang)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Language> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl IntoIterator for LanguageInventory {
    type Item = Language;
    type IntoIter = std::collections::btree_set::IntoIter<Language>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a LanguageInventory {
    type Item = &'a Language;
    type IntoIter = std::collections::btree_set::Iter<'a, Language>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl FromIterator<Language> for LanguageInventory {
    fn from_iter<I: IntoIterator<Item = Language>>(iter: I) -> Self {
        Self(BTreeSet::from_iter(iter))
    }
}

impl Extend<Language> for LanguageInventory {
    fn extend<I: IntoIterator<Item = Language>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

/// Immutable result of repository discovery (pipeline Stage 1 output).
///
/// Carries all discovered information about a repository at a point in time:
/// its identity, workspace structure, files, directories, manifests, and languages.
///
/// # Invariants
///
/// - The snapshot is immutable after construction.
/// - The snapshot always has a valid [`Repository`] and [`Workspace`].
/// - File and directory ordering is deterministic.
/// - Language inventory is consistent with file language annotations.
#[derive(Debug, Clone)]
pub struct RepositorySnapshot {
    repository: Repository,
    workspace: Workspace,
    file_inventory: FileInventory,
    directory_inventory: DirectoryInventory,
    manifest_inventory: ManifestInventory,
    language_inventory: LanguageInventory,
}

impl RepositorySnapshot {
    pub fn repository(&self) -> &Repository {
        &self.repository
    }

    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    pub fn files(&self) -> &[RepositoryFile] {
        self.file_inventory.as_slice()
    }

    pub fn directories(&self) -> &[RepositoryDirectory] {
        self.directory_inventory.as_slice()
    }

    pub fn manifests(&self) -> &[Manifest] {
        self.manifest_inventory.as_slice()
    }

    pub fn languages(&self) -> &BTreeSet<Language> {
        &self.language_inventory.0
    }

    pub fn file_inventory(&self) -> &FileInventory {
        &self.file_inventory
    }

    pub fn directory_inventory(&self) -> &DirectoryInventory {
        &self.directory_inventory
    }

    pub fn manifest_inventory(&self) -> &ManifestInventory {
        &self.manifest_inventory
    }

    pub fn language_inventory(&self) -> &LanguageInventory {
        &self.language_inventory
    }
}

/// Builder for assembling a [`RepositorySnapshot`].
///
/// # Required Fields
///
/// - [`repository`](Self::repository)
/// - [`workspace`](Self::workspace)
///
/// Other fields default to empty inventories. If [`language_inventory`](Self::language_inventory)
/// is not provided, it is derived from file language annotations.
///
/// # Invariants
///
/// - [`build`](Self::build) fails if repository or workspace is missing.
/// - Unset inventories default to empty (never `None`).
#[derive(Debug)]
pub struct SnapshotBuilder {
    repository: Option<Repository>,
    workspace: Option<Workspace>,
    files: Option<FileInventory>,
    directories: Option<DirectoryInventory>,
    manifests: Option<ManifestInventory>,
    languages: Option<LanguageInventory>,
}

impl SnapshotBuilder {
    pub fn new() -> Self {
        Self {
            repository: None,
            workspace: None,
            files: None,
            directories: None,
            manifests: None,
            languages: None,
        }
    }

    pub fn repository(mut self, repo: Repository) -> Self {
        self.repository = Some(repo);
        self
    }

    pub fn workspace(mut self, ws: Workspace) -> Self {
        self.workspace = Some(ws);
        self
    }

    pub fn file_inventory(mut self, inv: FileInventory) -> Self {
        self.files = Some(inv);
        self
    }

    pub fn directory_inventory(mut self, inv: DirectoryInventory) -> Self {
        self.directories = Some(inv);
        self
    }

    pub fn manifest_inventory(mut self, inv: ManifestInventory) -> Self {
        self.manifests = Some(inv);
        self
    }

    pub fn language_inventory(mut self, inv: LanguageInventory) -> Self {
        self.languages = Some(inv);
        self
    }

    pub fn build(self) -> Result<RepositorySnapshot, String> {
        let repository = self.repository.ok_or("repository is required")?;
        let workspace = self.workspace.ok_or("workspace is required")?;
        let file_inventory = self.files.unwrap_or_default();
        let directory_inventory = self.directories.unwrap_or_default();
        let manifest_inventory = self.manifests.unwrap_or_default();

        // Derive language inventory from files if not explicitly provided.
        let language_inventory = self
            .languages
            .unwrap_or_else(|| LanguageInventory::from_files(file_inventory.as_slice()));

        Ok(RepositorySnapshot {
            repository,
            workspace,
            file_inventory,
            directory_inventory,
            manifest_inventory,
            language_inventory,
        })
    }
}

impl Default for SnapshotBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::ManifestKind;
    use crate::workspace::WorkspaceKind;

    fn dummy_repo() -> Repository {
        Repository::new(".").unwrap()
    }

    #[test]
    fn snapshot_languages_collected_from_files() {
        let repo = dummy_repo();

        let file_inventory = FileInventory::new(vec![
            RepositoryFile {
                relative_path: PathBuf::from("main.rs"),
                metadata: FileMetadata::new(100, None),
                language: Some(Language::Rust),
            },
            RepositoryFile {
                relative_path: PathBuf::from("lib.rs"),
                metadata: FileMetadata::new(200, None),
                language: Some(Language::Rust),
            },
            RepositoryFile {
                relative_path: PathBuf::from("README.md"),
                metadata: FileMetadata::new(50, None),
                language: Some(Language::Markdown),
            },
            RepositoryFile {
                relative_path: PathBuf::from("no_ext"),
                metadata: FileMetadata::new(30, None),
                language: None,
            },
        ]);

        let snapshot = SnapshotBuilder::new()
            .repository(repo)
            .workspace(Workspace {
                kind: WorkspaceKind::None,
            })
            .file_inventory(file_inventory)
            .directory_inventory(DirectoryInventory::new(Vec::new()))
            .manifest_inventory(ManifestInventory::new())
            .build()
            .unwrap();

        let langs: BTreeSet<_> = snapshot.languages().iter().map(|l| l.to_string()).collect();
        let mut expected = BTreeSet::new();
        expected.insert("Rust".to_string());
        expected.insert("Markdown".to_string());
        assert_eq!(langs, expected);
    }

    #[test]
    fn snapshot_manifests_preserved() {
        let repo = dummy_repo();
        let manifest_inventory = ManifestInventory::from_manifests(vec![Manifest::new(
            PathBuf::from("Cargo.toml"),
            ManifestKind::CargoManifest,
        )]);

        let snapshot = SnapshotBuilder::new()
            .repository(repo)
            .workspace(Workspace {
                kind: WorkspaceKind::None,
            })
            .file_inventory(FileInventory::new(Vec::new()))
            .directory_inventory(DirectoryInventory::new(Vec::new()))
            .manifest_inventory(manifest_inventory)
            .build()
            .unwrap();

        assert_eq!(snapshot.manifests().len(), 1);
        assert_eq!(snapshot.manifests()[0].kind(), &ManifestKind::CargoManifest);
    }

    #[test]
    fn snapshot_file_accessors() {
        let f = RepositoryFile {
            relative_path: PathBuf::from("src/lib.rs"),
            metadata: FileMetadata::new(1024, None),
            language: Some(Language::Rust),
        };

        assert_eq!(f.relative_path(), Path::new("src/lib.rs"));
        assert_eq!(f.size(), 1024);
        assert_eq!(f.modified(), None);
        assert_eq!(f.language(), Some(&Language::Rust));
    }

    #[test]
    fn snapshot_directory_accessors() {
        let d = RepositoryDirectory {
            relative_path: PathBuf::from("src"),
        };
        assert_eq!(d.relative_path(), Path::new("src"));
    }

    #[test]
    fn snapshot_builder_derives_languages_from_files() {
        let repo = dummy_repo();
        let files = FileInventory::new(vec![RepositoryFile {
            relative_path: PathBuf::from("main.rs"),
            metadata: FileMetadata::new(100, None),
            language: Some(Language::Rust),
        }]);

        let snapshot = SnapshotBuilder::new()
            .repository(repo)
            .workspace(Workspace {
                kind: WorkspaceKind::None,
            })
            .file_inventory(files)
            .build()
            .unwrap();

        assert_eq!(snapshot.languages().len(), 1);
        assert!(snapshot.languages().contains(&Language::Rust));
    }

    #[test]
    fn snapshot_builder_missing_repository_fails() {
        let result = SnapshotBuilder::new()
            .workspace(Workspace {
                kind: WorkspaceKind::None,
            })
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn file_metadata() {
        let meta = FileMetadata::new(42, None);
        assert_eq!(meta.size(), 42);
        assert_eq!(meta.modified(), None);
    }

    #[test]
    fn manifest_inventory() {
        let mut inv = ManifestInventory::new();
        assert!(inv.is_empty());
        inv.push(Manifest::new(
            PathBuf::from("Cargo.toml"),
            ManifestKind::CargoManifest,
        ));
        assert_eq!(inv.len(), 1);
        assert!(inv.contains_path(Path::new("Cargo.toml")));
        assert!(!inv.contains_path(Path::new("other.toml")));
    }

    #[test]
    fn language_inventory() {
        let inv =
            LanguageInventory::from_languages(BTreeSet::from([Language::Rust, Language::Toml]));
        assert_eq!(inv.len(), 2);
        assert!(inv.contains(&Language::Rust));
        assert!(!inv.contains(&Language::Python));
    }

    #[test]
    fn file_inventory_default_is_empty() {
        let inv = FileInventory::default();
        assert!(inv.is_empty());
    }

    #[test]
    fn file_inventory_into_iter() {
        let files = vec![RepositoryFile {
            relative_path: PathBuf::from("a.rs"),
            metadata: FileMetadata::new(1, None),
            language: None,
        }];
        let inv = FileInventory::new(files);
        let count = inv.into_iter().count();
        assert_eq!(count, 1);
    }

    #[test]
    fn file_inventory_from_iterator() {
        let files = vec![RepositoryFile {
            relative_path: PathBuf::from("a.rs"),
            metadata: FileMetadata::new(1, None),
            language: None,
        }];
        let inv = FileInventory::from_iter(files);
        assert_eq!(inv.len(), 1);
    }

    #[test]
    fn file_inventory_extend() {
        let mut inv = FileInventory::default();
        inv.extend(vec![RepositoryFile {
            relative_path: PathBuf::from("b.rs"),
            metadata: FileMetadata::new(2, None),
            language: None,
        }]);
        assert_eq!(inv.len(), 1);
    }

    #[test]
    fn file_inventory_as_ref() {
        let inv = FileInventory::new(vec![RepositoryFile {
            relative_path: PathBuf::from("a.rs"),
            metadata: FileMetadata::new(1, None),
            language: None,
        }]);
        let slice: &[RepositoryFile] = inv.as_ref();
        assert_eq!(slice.len(), 1);
    }

    #[test]
    fn directory_inventory_default_is_empty() {
        let inv = DirectoryInventory::default();
        assert!(inv.is_empty());
    }

    #[test]
    fn manifest_inventory_into_iter() {
        let inv = ManifestInventory::from_manifests(vec![Manifest::new(
            PathBuf::from("Cargo.toml"),
            ManifestKind::CargoManifest,
        )]);
        let count = inv.into_iter().count();
        assert_eq!(count, 1);
    }

    #[test]
    fn manifest_inventory_from_iterator() {
        let manifests = vec![Manifest::new(
            PathBuf::from("Cargo.toml"),
            ManifestKind::CargoManifest,
        )];
        let inv = ManifestInventory::from_iter(manifests);
        assert_eq!(inv.len(), 1);
    }

    #[test]
    fn language_inventory_into_iter() {
        let inv = LanguageInventory::from_languages(BTreeSet::from([Language::Rust]));
        let langs: Vec<_> = inv.into_iter().collect();
        assert_eq!(langs, vec![Language::Rust]);
    }

    #[test]
    fn language_inventory_from_iterator() {
        let inv = LanguageInventory::from_iter(vec![Language::Rust, Language::Toml]);
        assert_eq!(inv.len(), 2);
    }

    #[test]
    fn language_inventory_extend() {
        let mut inv = LanguageInventory::default();
        inv.extend(vec![Language::Rust]);
        assert!(inv.contains(&Language::Rust));
    }
}
