use crate::extraction::diagnostic::ExtractionDiagnostic;
use crate::extraction::model::Entity;

/// Reason extraction was skipped for a syntax tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    /// No extractor is registered for this language.
    UnsupportedLanguage,
    /// The syntax tree contains errors that prevent extraction.
    InvalidSyntax,
}

/// Outcome of extracting facts from a single syntax tree.
#[derive(Debug, Clone)]
pub enum ExtractionOutcome {
    /// All entities extracted successfully.
    Success(Vec<Entity>),
    /// Entities extracted with diagnostics (recoverable issues).
    Partial(Vec<Entity>, Vec<ExtractionDiagnostic>),
    /// Extraction was skipped intentionally.
    Skipped(SkipReason),
    /// Extraction failed entirely.
    Failed(Vec<ExtractionDiagnostic>),
}
