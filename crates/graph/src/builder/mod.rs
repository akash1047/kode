//! Graph construction orchestration.
//!
//! [`GraphBuilder`] is the single entry point for building a [`KnowledgeGraph`].
//! It orchestrates node construction, relationship derivation, indexing, and
//! validation in a deterministic, single-pass transformation.
//!
//! # Pipeline
//!
//! ```text
//! build()
//!
//! ↓
//!
//! Create structural nodes (repository, workspace, file)
//!
//! ↓
//!
//! Create entity nodes (from extracted facts)
//!
//! ↓
//!
//! Sort nodes by GraphNodeId
//!
//! ↓
//!
//! Build node index (detects duplicates)
//!
//! ↓
//!
//! Build relationships
//!
//! ↓
//!
//! Build edge index
//!
//! ↓
//!
//! Validate
//!
//! ↓
//!
//! KnowledgeGraph
//! ```
//!
//! # Ownership Boundaries
//!
//! * [`RepositoryContext`] is constructed **externally** and passed in.
//! * [`GraphBuilder`] performs zero repository discovery.
//! * Structural IDs are generated exactly once by `structural` and
//!   consumed by `relationship_builder` via [`StructuralLookup`].
//!
//! # Module Responsibilities
//!
//! * `identity` — [`GraphNodeId`] and [`GraphEvidence`] construction
//! * `structural` — repository, workspace, and file node creation
//! * `node_builder` — entity node creation from extracted facts
//! * `relationship_builder` — relationship derivation
//! * `index` — lookup and edge index construction
//! * `mod.rs` — orchestration only

pub mod context;
pub(crate) mod identity;
pub(crate) mod index;
pub(crate) mod node_builder;
pub(crate) mod relationship_builder;
pub(crate) mod structural;

use kode_analysis::extraction::RepositoryFacts;

use crate::model::KnowledgeGraph;
use crate::validator::ValidationError;

use self::context::RepositoryContext;
use self::index::{build_edge_index, build_node_index};
use self::node_builder::push_entity_nodes;
use self::relationship_builder::build_relationships;
use self::structural::push_structural_nodes;

/// Accumulated state during graph construction.
///
/// Enables the builder pipeline to mutate a single structure instead of
/// passing independent vectors and maps between phases. Only the final
/// step extracts an immutable [`KnowledgeGraph`].
///
/// # Lifecycle
///
/// 1. Nodes are pushed into [`nodes`](GraphBuildState::nodes).
/// 2. Nodes are sorted in place.
/// 3. [`node_by_id`](GraphBuildState::node_by_id) is built.
/// 4. Relationships are pushed into [`relationships`](GraphBuildState::relationships).
/// 5. Edge indexes are built.
/// 6. [`into_graph`](GraphBuildState::into_graph) finalizes the
///    [`KnowledgeGraph`].
pub(crate) struct GraphBuildState {
    pub(crate) nodes: Vec<crate::model::Node>,
    pub(crate) relationships: Vec<crate::model::Relationship>,
    pub(crate) node_by_id: std::collections::BTreeMap<crate::model::GraphNodeId, usize>,
    pub(crate) outgoing: Vec<Vec<usize>>,
    pub(crate) incoming: Vec<Vec<usize>>,
}

impl GraphBuildState {
    /// Create an empty build state.
    pub(crate) fn new() -> Self {
        Self {
            nodes: Vec::new(),
            relationships: Vec::new(),
            node_by_id: std::collections::BTreeMap::new(),
            outgoing: Vec::new(),
            incoming: Vec::new(),
        }
    }

    /// Sort nodes deterministically by [`GraphNodeId`].
    pub(crate) fn sort_nodes(&mut self) {
        self.nodes.sort_by(|a, b| a.id().cmp(b.id()));
    }

    /// Build the node index (detects duplicates).
    pub(crate) fn build_node_index(&mut self) -> Result<(), Vec<ValidationError>> {
        self.node_by_id = build_node_index(&self.nodes)?;
        Ok(())
    }

    /// Build edge index structures.
    pub(crate) fn build_edge_index(&mut self) {
        let n = self.nodes.len();
        let (outgoing, incoming) = build_edge_index(n, &self.relationships, &self.node_by_id);
        self.outgoing = outgoing;
        self.incoming = incoming;
    }

    /// Finalize into an immutable [`KnowledgeGraph`].
    pub(crate) fn into_graph(self) -> KnowledgeGraph {
        KnowledgeGraph::new(
            self.nodes,
            self.relationships,
            self.node_by_id,
            self.outgoing,
            self.incoming,
        )
    }
}

/// Single entry point for graph construction.
///
/// # Usage
///
/// ```ignore
/// let ctx = RepositoryContext::from_facts(&facts);
/// let graph = GraphBuilder::build(&facts, &ctx)?;
/// ```
///
/// Construction is deterministic: repeated calls with identical
/// [`RepositoryFacts`] and [`RepositoryContext`] produce identical
/// [`KnowledgeGraph`] instances.
pub struct GraphBuilder;

impl GraphBuilder {
    /// Build a validated [`KnowledgeGraph`] from [`RepositoryFacts`] and
    /// [`RepositoryContext`].
    ///
    /// The builder:
    ///
    /// 1. Creates structural nodes (repository, workspace, file).
    /// 2. Creates entity nodes (one per extracted entity).
    /// 3. Sorts all nodes deterministically by [`GraphNodeId`].
    /// 4. Builds index structures.
    /// 5. Derives relationships (contains, declares, defines).
    /// 6. Builds edge index structures.
    /// 7. Validates graph invariants.
    ///
    /// [`RepositoryContext`] is provided externally — the builder performs
    /// no repository discovery.
    ///
    /// Returns the graph on success, or all validation errors on failure.
    pub fn build(
        facts: &RepositoryFacts,
        ctx: &RepositoryContext,
    ) -> Result<KnowledgeGraph, Vec<ValidationError>> {
        let mut state = GraphBuildState::new();

        // --- Step 1: Create structural nodes + structural lookup ---
        let structural = push_structural_nodes(ctx, &mut state.nodes);

        // --- Step 2: Create entity nodes ---
        push_entity_nodes(facts, &mut state.nodes);

        // --- Step 3: Sort deterministically by GraphNodeId ---
        state.sort_nodes();

        // --- Step 4: Build node index (detects duplicates) ---
        state.build_node_index()?;

        // --- Step 5: Build relationships ---
        state.relationships = build_relationships(facts, &state.node_by_id, ctx, &structural);

        // --- Step 6: Build edge index structures ---
        state.build_edge_index();

        let graph = state.into_graph();

        // --- Step 7: Validate ---
        crate::validator::GraphValidator::validate(&graph)?;

        Ok(graph)
    }
}
