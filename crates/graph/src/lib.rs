#![allow(unused_crate_dependencies)]

//! Knowledge Graph: canonical representation of repository knowledge.
//!
//! Owns the graph data structure (Stage 4) that connects repository facts
//! into an immutable, validated, traversable model.
//!
//! # Pipeline Position
//!
//! ```text
//! RepositoryFacts (Stage 3)
//! RepositoryContext (Stage 4 adapter —> Stage 5 Acquisition)
//!         ↓
//!   GraphBuilder         ← performs zero repository discovery
//!         ↓
//!   GraphValidator
//!         ↓
//!   KnowledgeGraph  ← you are here
//!         ↓
//!   Storage / Query / Analysis (future stages)
//! ```
//!
//! # Responsibilities
//!
//! - Graph construction from [`RepositoryFacts`](kode_analysis::extraction::RepositoryFacts)
//!   and [`RepositoryContext`]
//! - Structural node identity (repository, workspace, file) via [`GraphNodeId`]
//! - Entity node identity via [`GraphNodeId::Entity`] wrapping
//!   [`EntityId`](kode_analysis::extraction::EntityId)
//! - Evidence attachment via [`GraphEvidence`] (typed [`StructuralEvidence`](model::StructuralEvidence)
//!   for synthetic nodes)
//! - Structural validation before graph release
//! - Read-only traversal (lookup by ID, kind, incident edges)
//!
//! # Ownership Boundaries
//!
//! * Repository adaptation is **external** — [`RepositoryContext`] is the
//!   sole owner of temporary metadata derivation. [`GraphBuilder`] performs
//!   zero repository discovery.
//! * Structural IDs are generated **exactly once** during node construction
//!   and cached in a [`StructuralLookup`] for relationship building.
//! * [`GraphBuildState`] replaces independent vectors/maps with a single
//!   mutable state object during construction.
//!
//! # Invariants
//!
//! - The graph is immutable after construction.
//! - Every node has a unique [`GraphNodeId`].
//! - Every node has [`GraphEvidence`].
//! - Every relationship has [`GraphEvidence`].
//! - Structural nodes use [`GraphNodeId::Structural`] and
//!   [`GraphEvidence::Structural`](model::GraphEvidence::Structural).
//! - Entity nodes use [`GraphNodeId::Entity`] and
//!   [`GraphEvidence::Source`](model::GraphEvidence::Source).
//! - Iteration order is deterministic.
//! - Invariants are enforced at the type level — invalid combinations
//!   cannot be constructed even in release builds.
//!
//! # Extension Points
//!
//! - [`NodeMetadata`] and [`RelationshipMetadata`] carry optional fields;
//!   new fields can be added without changing the public API shape.
//! - New [`NodeKind`] variants can be added as new language entity kinds
//!   are introduced.
//! - [`RelationshipKind::Calls`] edges are built during graph construction
//!   by resolving call sites extracted from function bodies (name-based).

use thiserror as _;

pub mod builder;
pub mod model;
pub mod serialization;
pub mod validator;

pub use builder::context::RepositoryContext;
pub use builder::GraphBuilder;
pub use model::*;
pub use serialization::{
    dtos_to_graph, graph_to_dot, graph_to_dtos, graph_to_graphml, NodeDto, RelationshipDto,
    SerializationError,
};
pub use validator::GraphValidator;
pub use validator::ValidationError;

// Re-export types from analysis that are part of the graph's public API
pub use kode_analysis::extraction::{EntityId, Evidence, Visibility};
// Re-export Language from acquisition as it is part of evidence construction
pub use kode_acquisition::Language;
