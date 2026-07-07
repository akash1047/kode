//! Graph-level identity and evidence helpers.
//!
//! Provides helper functions for constructing [`StructuralNodeId`] values
//! during graph building.

use std::path::Path;

use crate::model::{StructuralNodeId, StructuralNodeKind};

/// Build a [`StructuralNodeId`] for a structural node.
pub fn structural_id(kind: StructuralNodeKind, path: &Path, name: &str) -> StructuralNodeId {
    StructuralNodeId::from_parts(kind, path, name)
}
