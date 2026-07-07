//! Repository context for graph construction.
//!
//! [`RepositoryContext`] encapsulates all repository-level metadata required
//! to build a graph. It is the sole owner of temporary metadata derivation
//! from extracted facts, isolating the discovery logic so that future
//! Acquisition metadata integration requires only changing this one adapter.
//!
//! # Current State
//!
//! The context is constructed from [`RepositoryFacts`] using temporary
//! heuristics. In Stage 5, Acquisition will provide this metadata directly,
//! making the adapter functions in this module obsolete.
//!
//! # Ownership Boundary
//!
//! * [`RepositoryContext`] is constructed externally from [`GraphBuilder`].
//! * [`GraphBuilder`] receives `&RepositoryContext` — it performs no
//!   repository discovery of its own.
//! * All temporary derivation is behind this single adapter.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use kode_analysis::extraction::RepositoryFacts;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Hash name for repository structural identity computation.
pub const REPOSITORY_HASH_NAME: &str = "repo";

/// Default workspace name for repositories without explicit workspace metadata.
pub const DEFAULT_WORKSPACE_NAME: &str = "default";

/// Display name for the repository structural node.
pub const REPOSITORY_NODE_NAME: &str = "repository";

/// Repository-level metadata consumed during graph construction.
///
/// Carries the repository root, workspace name, and the set of source
/// files discovered from extracted entity evidence.
///
/// This is the **sole owner** of temporary repository metadata derivation.
/// Stage 5 replaces only this type's construction.
pub struct RepositoryContext {
    /// Unique source file paths collected from entity evidence.
    source_files: BTreeSet<PathBuf>,
    /// Computed repository root (common ancestor of all source files).
    root_path: PathBuf,
    /// The workspace name.
    workspace_name: String,
}

impl RepositoryContext {
    /// Construct a context from extracted facts.
    ///
    /// This performs the **temporary** discovery that will be replaced
    /// by Acquisition-provided metadata in Stage 5.
    ///
    /// # Temporary Adapters
    ///
    /// * [`collect_source_files`] — collects unique source files from entity evidence.
    /// * [`compute_root_path`] — computes the common ancestor path.
    ///
    /// Both live in this module and should be replaced by Acquisition metadata.
    pub fn from_facts(facts: &RepositoryFacts) -> Self {
        let source_files = collect_source_files(facts);
        let root_path = compute_root_path(&source_files);
        Self::new(root_path, workspace_name(), source_files)
    }

    /// Construct a context with explicitly provided metadata.
    ///
    /// This is the canonical constructor. Once Acquisition provides real
    /// metadata in Stage 5, only callers of this method need to change.
    pub fn new(
        root_path: PathBuf,
        workspace_name: String,
        source_files: BTreeSet<PathBuf>,
    ) -> Self {
        Self {
            source_files,
            root_path,
            workspace_name,
        }
    }

    pub fn source_files(&self) -> &BTreeSet<PathBuf> {
        &self.source_files
    }

    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    pub fn workspace_name(&self) -> &str {
        &self.workspace_name
    }
}

fn workspace_name() -> String {
    DEFAULT_WORKSPACE_NAME.to_string()
}

/// Collect unique source files from entity evidence.
fn collect_source_files(facts: &RepositoryFacts) -> BTreeSet<PathBuf> {
    let mut files = BTreeSet::new();

    macro_rules! collect_from {
        ($accessor:ident) => {
            for entity in facts.$accessor() {
                files.insert(entity.evidence().source_file().to_path_buf());
            }
        };
    }

    collect_from!(modules);
    collect_from!(functions);
    collect_from!(structs);
    collect_from!(enums);
    collect_from!(traits);
    collect_from!(impl_blocks);
    collect_from!(type_aliases);
    collect_from!(constants);
    collect_from!(statics);
    collect_from!(imports);
    collect_from!(exports);

    files
}

/// Compute common ancestor path from a set of source files.
fn compute_root_path(paths: &BTreeSet<PathBuf>) -> PathBuf {
    if paths.is_empty() {
        return PathBuf::from(".");
    }

    let mut iter = paths.iter();

    let first_parent = iter
        .next()
        .and_then(|p| p.parent())
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let mut root = first_parent;
    for path in iter {
        if let Some(parent) = path.parent() {
            root = common_ancestor(&root, parent);
        }
    }
    root
}

fn common_ancestor(a: &Path, b: &Path) -> PathBuf {
    let a_components: Vec<_> = a.components().collect();
    let b_components: Vec<_> = b.components().collect();
    let mut result = PathBuf::new();
    for (ac, bc) in a_components.iter().zip(b_components.iter()) {
        if ac == bc {
            result.push(ac.as_os_str());
        } else {
            break;
        }
    }
    if result.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        result
    }
}
