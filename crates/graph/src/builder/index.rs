//! Index construction for graph traversal.
//!
//! Builds the lookup indexes used by [`KnowledgeGraph`] for O(log n) node
//! lookup by [`GraphNodeId`] and O(1) edge traversal.
//!
//! # Invariants
//!
//! * Detects duplicate [`GraphNodeId`] values during index construction.
//! * Edge indexes are built after all nodes are placed at their final indices.
//! * All index structures are deterministic given the same node and
//!   relationship vectors.

use std::collections::BTreeMap;

use crate::model::{GraphNodeId, Relationship};
use crate::validator::ValidationError;

/// Build the node-by-id lookup index.
///
/// Returns an error if a duplicate [`GraphNodeId`] is detected.
pub fn build_node_index(
    nodes: &[crate::model::Node],
) -> Result<BTreeMap<GraphNodeId, usize>, Vec<ValidationError>> {
    let mut node_by_id = BTreeMap::new();
    for (i, node) in nodes.iter().enumerate() {
        if node_by_id.contains_key(node.id()) {
            return Err(vec![ValidationError::DuplicateNodeId { id: *node.id() }]);
        }
        node_by_id.insert(*node.id(), i);
    }
    Ok(node_by_id)
}

/// Build outgoing and incoming edge index structures.
///
/// Both indexes are `Vec<Vec<usize>>` indexed by node position (0..n).
pub fn build_edge_index(
    n: usize,
    relationships: &[Relationship],
    node_by_id: &BTreeMap<GraphNodeId, usize>,
) -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
    let mut outgoing = vec![Vec::new(); n];
    let mut incoming = vec![Vec::new(); n];
    for (i, rel) in relationships.iter().enumerate() {
        if let Some(&src_idx) = node_by_id.get(rel.source()) {
            outgoing[src_idx].push(i);
        }
        if let Some(&tgt_idx) = node_by_id.get(rel.target()) {
            incoming[tgt_idx].push(i);
        }
    }
    (outgoing, incoming)
}
