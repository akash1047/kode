//! Structural node construction.
//!
//! Creates synthetic graph nodes for repository, workspace, and source files.
//! These nodes have no corresponding extracted entity — they exist purely to
//! organise the graph hierarchy.
//!
//! # Invariants
//!
//! * Structural nodes always use [`GraphNodeId::Structural`].
//! * Structural nodes always carry [`GraphEvidence::Structural`].
//! * No structural node fabricates parser evidence.
//! * No structural node wraps an [`EntityId`].
//! * Structural nodes are deterministic across runs.
//! * Repository discovery is performed by [`RepositoryContext`], not this
//!   module.
//! * Structural IDs are generated exactly once and returned via
//!   [`StructuralLookup`] for use by relationship construction — no
//!   duplicate hashing occurs.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::model::{
    GraphNodeId, Node, NodeKind, NodeMetadata, StructuralEvidence, StructuralNodeKind,
};

use super::context::{RepositoryContext, REPOSITORY_HASH_NAME};
use super::identity::structural_id;

/// Pre-computed structural node identities.
///
/// Generated once during structural node construction and consumed by
/// relationship construction to avoid recomputing structural hashes.
#[derive(Debug, Clone)]
pub struct StructuralLookup {
    /// The repository node's [`GraphNodeId`].
    pub repo_id: GraphNodeId,
    /// The workspace node's [`GraphNodeId`].
    pub workspace_id: GraphNodeId,
    /// File node IDs indexed by path.
    pub file_by_path: BTreeMap<PathBuf, GraphNodeId>,
}

/// Push structural nodes (repository, workspace, files) into the node vector
/// and return a [`StructuralLookup`] for use by relationship construction.
///
/// Consumes repository metadata from the [`RepositoryContext`] rather than
/// performing any discovery itself.
pub fn push_structural_nodes(ctx: &RepositoryContext, nodes: &mut Vec<Node>) -> StructuralLookup {
    // Repository node
    let repo_id = GraphNodeId::Structural(structural_id(
        StructuralNodeKind::Repository,
        Path::new(""),
        REPOSITORY_HASH_NAME,
    ));
    nodes.push(Node::structural(
        structural_id(
            StructuralNodeKind::Repository,
            Path::new(""),
            REPOSITORY_HASH_NAME,
        ),
        NodeKind::Repository,
        "repository",
        NodeMetadata::new(None, None),
        StructuralEvidence::Repository {
            root: ctx.root_path().to_path_buf(),
        },
    ));

    // Workspace node
    let workspace_id = GraphNodeId::Structural(structural_id(
        StructuralNodeKind::Workspace,
        Path::new(""),
        ctx.workspace_name(),
    ));
    nodes.push(Node::structural(
        structural_id(
            StructuralNodeKind::Workspace,
            Path::new(""),
            ctx.workspace_name(),
        ),
        NodeKind::Workspace,
        ctx.workspace_name(),
        NodeMetadata::new(None, None),
        StructuralEvidence::Workspace {
            name: ctx.workspace_name().to_string(),
        },
    ));

    // File nodes
    let mut file_by_path = BTreeMap::new();
    for file_path in ctx.source_files() {
        let file_id = GraphNodeId::Structural(structural_id(
            StructuralNodeKind::File,
            file_path,
            "",
        ));
        nodes.push(Node::structural(
            structural_id(StructuralNodeKind::File, file_path, ""),
            NodeKind::File,
            file_path.display().to_string(),
            NodeMetadata::new(None, None),
            StructuralEvidence::File {
                path: file_path.clone(),
            },
        ));
        file_by_path.insert(file_path.clone(), file_id);
    }

    StructuralLookup {
        repo_id,
        workspace_id,
        file_by_path,
    }
}
