use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use kode_acquisition::RepositorySnapshot;

/// Immutable collection of source file contents loaded from disk.
///
/// Acts as the bridge between the filesystem (Stage 1) and parsing (Stage 2).
/// Source text is stored as [`Arc<str>`] to enable zero-copy sharing with
/// multiple [`SyntaxTree`] instances.
///
/// # Lifecycle
///
/// Created once per pipeline run, consumed by [`ParsingOrchestrator`].
/// All filesystem access happens during construction; the orchestrator
/// never reads from disk.
///
/// [`SyntaxTree`]: super::SyntaxTree
/// [`ParsingOrchestrator`]: super::ParsingOrchestrator
#[derive(Debug, Clone)]
pub struct SourceInventory {
    sources: HashMap<PathBuf, Arc<str>>,
}

impl SourceInventory {
    pub fn new(sources: HashMap<PathBuf, Arc<str>>) -> Self {
        Self { sources }
    }

    /// Load all source files from a snapshot into memory.
    ///
    /// # Errors
    ///
    /// Returns the first I/O error encountered. Partial loads are not
    /// supported; if any file cannot be read, the entire operation fails.
    pub fn from_snapshot(snapshot: &RepositorySnapshot) -> Result<Self, std::io::Error> {
        Self::from_snapshot_paths(snapshot, None)
    }

    /// Load source files, optionally restricted to `only` relative paths.
    pub fn from_snapshot_paths(
        snapshot: &RepositorySnapshot,
        only: Option<&std::collections::HashSet<PathBuf>>,
    ) -> Result<Self, std::io::Error> {
        let root = snapshot.repository().root();
        let mut sources = HashMap::with_capacity(snapshot.files().len());

        for file in snapshot.files() {
            if let Some(filter) = only {
                if !filter.contains(file.relative_path()) {
                    continue;
                }
            }
            let full_path = root.join(file.relative_path());
            let content = std::fs::read_to_string(&full_path)?;
            sources.insert(
                file.relative_path().to_path_buf(),
                Arc::from(content.as_str()),
            );
        }

        Ok(Self { sources })
    }

    pub fn get(&self, path: &Path) -> Option<&Arc<str>> {
        self.sources.get(path)
    }

    pub fn contains(&self, path: &Path) -> bool {
        self.sources.contains_key(path)
    }

    pub fn len(&self) -> usize {
        self.sources.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}
