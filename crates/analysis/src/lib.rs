#![allow(unused_crate_dependencies)]

//! Analysis subsystem: derives higher-level repository knowledge from the Knowledge Graph.
//!
//! Provides pipeline Stage 2 (Parsing) and Stage 3 (Fact Extraction).
//!
//! # Parsing (Stage 2)
//!
//! Transforms `RepositorySnapshot` and `SourceInventory` into `SyntaxTreeInventory`.
//!
//! - `ParserRegistry` — manages parser implementations with language-keyed dispatch
//! - `ParsingOrchestrator` — transforms `RepositorySnapshot` and `SourceInventory` into `SyntaxTreeInventory`
//! - `Parser` trait — implemented by each language-specific parser
//! - `SyntaxTree` — immutable domain artifact for a single parsed file
//! - `SyntaxTreeInventory` — immutable domain artifact for the entire repository
//! - `SourceInventory` — immutable source text storage
//! - `ParseOutcome` — discriminates success, recovery, skipped, and failure
//!
//! # Fact Extraction (Stage 3)
//!
//! Transforms `SyntaxTreeInventory` into `RepositoryFacts`.
//!
//! - `ExtractorRegistry` — manages extractor implementations with language-keyed dispatch
//! - `ExtractionOrchestrator` — transforms `SyntaxTreeInventory` into `RepositoryFacts`
//! - `Extractor` trait — implemented by each language-specific extractor
//! - `RepositoryFacts` — immutable, complete extracted semantic model for a repository
//! - `EntityId` — stable, deterministic identifier for every extracted entity
//! - `Evidence` — source location evidence attached to every entity

#[cfg(test)]
use tempfile as _;

pub mod extraction;
pub mod parsing;
