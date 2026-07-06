//! Knowledge Graph: canonical representation of repository knowledge.
//!
//! Owns the graph data structure that connects repository facts into a
//! traversable, queryable model. Downstream consumers (analysis, query)
//! read from the graph but never mutate it.
//!
//! # Responsibilities
//!
//! - Graph construction from acquisition artifacts
//! - Node identity and ownership
//! - Evidence attachment to graph edges
//!
//! # Invariants
//!
//! - The graph is append-only during a single revision.
//! - Graph revisions are immutable once committed.
//! - Node identity is stable across revisions.
