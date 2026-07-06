use crate::parsing::diagnostic::Diagnostic;
use crate::parsing::syntax::SyntaxTree;

/// Reason parsing was skipped for a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    /// No parser is registered for this file's language.
    UnsupportedLanguage,
}

/// Discriminated result of parsing a single file.
///
/// # Variants
///
/// - [`Success`](ParseOutcome::Success): valid syntax, no errors.
/// - [`Recovered`](ParseOutcome::Recovered): parse completed with error recovery.
/// - [`Skipped`](ParseOutcome::Skipped): intentionally skipped (e.g., unsupported language).
/// - [`Failed`](ParseOutcome::Failed): parse could not complete.
///
/// Both `Success` and `Recovered` carry a full [`SyntaxTree`]. Downstream
/// consumers can distinguish them to decide whether to trust the tree.
#[derive(Debug, Clone)]
pub enum ParseOutcome {
    Success(SyntaxTree),
    Recovered(SyntaxTree),
    Skipped(SkipReason),
    Failed(Vec<Diagnostic>),
}
