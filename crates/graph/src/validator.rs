//! Graph validation infrastructure.
//!
//! [`GraphValidator`] focuses on graph topology invariants — the things
//! that typed constructors cannot guarantee at the type level.
//!
//! # Invariants Validated
//!
//! * Every node has a unique [`GraphNodeId`] (defensive check for externally
//!   constructed graphs — the builder guarantees uniqueness during index
//!   construction).
//! * Relationship endpoints refer to existing node IDs.
//! * Exactly one repository node exists.
//! * Exactly one workspace node exists.
//! * Structural evidence fields carry non-empty data.
//!
//! # Responsibility Boundary
//!
//! The duplicate ID check exists **only** as a defensive safety net.
//!
//! * [`GraphBuilder`] (via [`build_node_index`]) is responsible for
//!   preventing duplicate node IDs during construction.
//! * The validator's duplicate check protects against malformed graphs
//!   constructed outside the builder (e.g., via [`KnowledgeGraph::new`]).
//!
//! # What the Validator Does NOT Check
//!
//! * Identity/evidence consistency — typed constructors
//!   ([`Node::structural`]/[`Node::entity`]) guarantee the correct pairing
//!   at the type level.
//! * Evidence completeness — typed constructors guarantee that every element
//!   carries valid evidence.

use thiserror::Error;

use crate::model::GraphNodeId;

use crate::model::{GraphEvidence, KnowledgeGraph, NodeKind, RelationshipKind, StructuralEvidence};

/// A structured validation error.
///
/// Each variant carries enough context for downstream CLI reporting.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ValidationError {
    /// Two or more nodes share the same [`GraphNodeId`].
    #[error("duplicate node id: {id}")]
    DuplicateNodeId { id: GraphNodeId },

    /// A relationship references a source or target that does not exist.
    #[error("orphan relationship: {kind} from {from_id} to {to_id}")]
    OrphanRelationship {
        from_id: GraphNodeId,
        to_id: GraphNodeId,
        kind: RelationshipKind,
    },

    /// A structural evidence variant has empty data.
    #[error("structural evidence field is empty")]
    MissingStructuralEvidence,

    /// The repository has no structural root nodes.
    #[error("graph has no repository node")]
    MissingRepository,

    /// The repository has no workspace node.
    #[error("graph has no workspace node")]
    MissingWorkspace,

    /// The graph contains multiple repository nodes.
    #[error("graph has multiple repository nodes")]
    MultipleRepositories,

    /// The graph contains multiple workspace nodes.
    #[error("graph has multiple workspace nodes")]
    MultipleWorkspaces,
}

/// Read-only graph validator.
///
/// Focuses on graph topology invariants only — constructor-level checks
/// are handled by typed constructors at compile time.
pub struct GraphValidator;

impl GraphValidator {
    /// Validate graph invariants.
    ///
    /// Returns `Ok(())` if all checks pass, or `Err` with every
    /// detected violation collected in deterministic order.
    pub fn validate(graph: &KnowledgeGraph) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // --- Duplicate node IDs (safety net — builder guarantees this) ---
        let mut seen = std::collections::BTreeMap::new();
        for node in graph.nodes() {
            if seen.contains_key(node.id()) {
                errors.push(ValidationError::DuplicateNodeId { id: *node.id() });
            }
            seen.insert(*node.id(), ());
        }

        // --- Structural evidence data quality ---
        for node in graph.nodes() {
            errors.extend(Self::check_node_evidence(node));
        }

        // --- Structural root checks ---
        errors.extend(Self::check_structural_roots(graph));

        // --- Orphan relationships + evidence ---
        for rel in graph.relationships() {
            errors.extend(Self::check_relationship_evidence(rel));

            if graph.node_by_id(rel.source()).is_none() || graph.node_by_id(rel.target()).is_none()
            {
                errors.push(ValidationError::OrphanRelationship {
                    from_id: *rel.source(),
                    to_id: *rel.target(),
                    kind: rel.kind(),
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn check_node_evidence(node: &crate::model::Node) -> Vec<ValidationError> {
        match node.evidence() {
            GraphEvidence::Source(_) => Vec::new(),
            GraphEvidence::Structural(sev) => check_structural_evidence(sev),
        }
    }

    fn check_relationship_evidence(rel: &crate::model::Relationship) -> Vec<ValidationError> {
        match rel.evidence() {
            GraphEvidence::Source(_) => Vec::new(),
            GraphEvidence::Structural(sev) => check_structural_evidence(sev),
        }
    }

    fn check_structural_roots(graph: &KnowledgeGraph) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let repo_count = graph.nodes_by_kind(NodeKind::Repository).count();
        let ws_count = graph.nodes_by_kind(NodeKind::Workspace).count();

        if repo_count == 0 {
            errors.push(ValidationError::MissingRepository);
        } else if repo_count > 1 {
            errors.push(ValidationError::MultipleRepositories);
        }

        if ws_count == 0 {
            errors.push(ValidationError::MissingWorkspace);
        } else if ws_count > 1 {
            errors.push(ValidationError::MultipleWorkspaces);
        }

        errors
    }
}

/// Validate that structural evidence carries non-empty data for its variant.
fn check_structural_evidence(sev: &StructuralEvidence) -> Vec<ValidationError> {
    match sev {
        StructuralEvidence::Repository { root } if root.as_os_str().is_empty() => {
            vec![ValidationError::MissingStructuralEvidence]
        }
        StructuralEvidence::Workspace { name } if name.is_empty() => {
            vec![ValidationError::MissingStructuralEvidence]
        }
        StructuralEvidence::File { path } if path.as_os_str().is_empty() => {
            vec![ValidationError::MissingStructuralEvidence]
        }
        _ => Vec::new(),
    }
}
