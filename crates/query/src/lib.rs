//! Query Engine: transforms repository knowledge into evidence-backed answers.
//!
//! Reads the Knowledge Graph and SyntaxTreeInventory to answer questions
//! about repository structure, definitions, references, and types.
//!
//! # Responsibilities
//!
//! - Evidence verification against live source code
//! - Graph traversal for definition and reference resolution
//! - Repository validation before query execution
//!
//! # Invariants
//!
//! - Queries never mutate the graph or storage.
//! - Every answer includes path:line citations.
//! - Queries are deterministic for the same graph revision.
