//! Fact Extraction subsystem (pipeline Stage 3).
//!
//! Transforms [`SyntaxTreeInventory`](crate::parsing::SyntaxTreeInventory) into
//! [`RepositoryFacts`] by selecting the appropriate [`Extractor`] for each
//! parsed syntax tree and executing it through the [`ExtractionOrchestrator`].
//!
//! # Architecture
//!
//! - [`Extractor`] trait — one per language, walks syntax trees to produce entities
//! - [`ExtractorRegistry`] — ordered collection of extractors, language-keyed dispatch
//! - [`ExtractionOrchestrator`] — pure transformation that drives the pipeline
//! - [`RepositoryFacts`] — immutable repository-wide collection of extracted entities
//! - [`EntityId`] — stable, deterministic identifier for every entity
//! - [`Evidence`] — source location evidence attached to every entity
//!
//! # Invariants
//!
//! - Every entity has an [`EntityId`] and [`Evidence`].
//! - Iteration order is deterministic (sorted by [`EntityId`]).
//! - Extraction continues past recoverable issues (reported as diagnostics).

mod cache;
mod diagnostic;
mod error;
mod extractor;
mod model;
mod orchestrator;
mod python;
mod registry;
mod result;
mod rust;

pub use cache::FactsCache;
pub use diagnostic::{ExtractionDiagnostic, Severity as ExtractionSeverity};
pub use error::Error;
pub use extractor::Extractor;
pub use model::*;
pub use orchestrator::ExtractionOrchestrator;
pub use python::PythonExtractor;
pub use registry::ExtractorRegistry;
pub use result::{ExtractionOutcome, SkipReason};
pub use rust::RustExtractor;
