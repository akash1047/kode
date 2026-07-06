//! Analysis subsystem: derives higher-level repository knowledge from the Knowledge Graph.
//!
//! Provides the pipeline Stage 2 (Parsing), which transforms `RepositorySnapshot`
//! and `SourceInventory` artifacts into `SyntaxTreeInventory` artifacts.
//!
//! # Parsing
//!
//! The `parsing` module implements the Parsing pipeline stage:
//!
//! - `ParserRegistry` — manages parser implementations with language-keyed dispatch
//! - `ParsingOrchestrator` — transforms `RepositorySnapshot` and `SourceInventory` into `SyntaxTreeInventory`
//! - `Parser` trait — implemented by each language-specific parser
//! - `SyntaxTree` — immutable domain artifact for a single parsed file
//! - `SyntaxTreeInventory` — immutable domain artifact for the entire repository
//! - `SourceInventory` — immutable source text storage
//! - `ParseOutcome` — discriminates success, recovery, skipped, and failure

#[cfg(test)]
use tempfile as _;

pub mod parsing;
