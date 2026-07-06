//! Parsing subsystem (pipeline Stage 2).
//!
//! Transforms [`RepositorySnapshot`](kode_acquisition::RepositorySnapshot) into
//! [`SyntaxTreeInventory`] by selecting the appropriate [`Parser`] for each
//! source file and executing it through the [`ParsingOrchestrator`].
//!
//! # Architecture
//!
//! - [`Parser`] trait — one per language, implemented for each grammar
//! - [`ParserRegistry`] — ordered collection of parsers, language-keyed dispatch
//! - [`ParsingOrchestrator`] — pure transformation that drives the pipeline
//! - [`SyntaxTree`] — immutable result of parsing one file
//! - [`SyntaxTreeInventory`] — immutable repository-wide collection of outcomes
//! - [`SourceInventory`] — immutable source text storage, loaded before parsing
//! - [`ParseOutcome`] — four-way discrimination (Success, Recovered, Skipped, Failed)

mod rust;
mod ts;

mod diagnostic;
mod error;
mod orchestrator;
mod parser;
mod registry;
mod result;
mod source;
mod syntax;

pub use diagnostic::{Diagnostic, Severity};
pub use error::Error;
pub use orchestrator::ParsingOrchestrator;
pub use parser::Parser;
pub use registry::ParserRegistry;
pub use result::{ParseOutcome, SkipReason};
pub use rust::RustParser;
pub use source::SourceInventory;
pub use syntax::{FileParseOutcome, SyntaxTree, SyntaxTreeInventory};
