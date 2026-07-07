#![allow(unused_crate_dependencies)]

//! Storage subsystem: persistence and caching for the Knowledge Graph.
//!
//! Manages revisioned storage of graph snapshots (Stage 5). Storage is a
//! **consumer** of the graph — it never initiates graph changes, performs
//! no analysis, and does not interpret the graph.
//!
//! # Pipeline Position
//!
//! ```text
//! KnowledgeGraph (Stage 4 — kode-graph)
//!         ↓
//!   Storage::persist(...)
//!         ↓
//!   GraphRevision
//!         ↓
//!   Analysis / Query / MCP (Stages 6+)
//! ```
//!
//! # Architecture
//!
//! * [`StorageBackend`] — abstract persistence trait (see [`backend`] module)
//! * [`SqliteBackend`] — default SQLite implementation (see [`sqlite`] module)
//! * [`RepositoryStorage`] — repository-scoped service (see [`service`] module)
//! * [`GraphRevision`] — immutable value representing one persisted state
//! * [`SchemaVersion`] / [`GraphVersion`] — independent versioning
//!
//! # Invariants
//!
//! * A committed revision is never mutated in place.
//! * Storage is a consumer of the graph; it never initiates graph changes.
//! * All writes are transactional — no partial persistence on failure.
//! * Repeated serialization of the same graph produces identical output.
//! * Graph ordering (sorted by [`GraphNodeId`]) is preserved across round-trips.
//! * All evidence is preserved during persistence and reload.

pub mod backend;
pub mod error;
pub mod model;
pub mod service;
pub mod sqlite;

pub use backend::StorageBackend;
pub use error::StorageError;
pub use model::*;
pub use service::RepositoryStorage;
pub use sqlite::SqliteBackend;
