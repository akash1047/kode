//! Storage subsystem: persistence and caching for the Knowledge Graph.
//!
//! Manages revisioned storage of graph snapshots. Provides incremental
//! update semantics so that only changed files are reprocessed.
//!
//! # Responsibilities
//!
//! - Graph revision persistence
//! - Incremental change detection
//! - Cache management for parsed artifacts
//!
//! # Invariants
//!
//! - A committed revision is never mutated in place.
//! - Storage is a consumer of the graph; it never initiates graph changes.
//! - Incremental updates preserve deterministic revision ordering.
