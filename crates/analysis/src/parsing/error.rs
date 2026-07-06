use thiserror::Error;

/// Parsing subsystem errors.
///
/// # Recovery Strategy
///
/// - [`ParserInit`](Error::ParserInit): parser setup failure; the language
///   grammar may be missing or corrupted. This is fatal for the affected language.
/// - [`UnsupportedLanguage`](Error::UnsupportedLanguage): no parser is available;
///   the caller should skip the file rather than aborting.
/// - [`Internal`](Error::Internal): unexpected parser failure (e.g., Tree-sitter
///   returned no tree). This is a bug.
///
/// These errors are internal to the parsing subsystem. Downstream consumers
/// receive parse outcomes via [`ParseOutcome`] instead.
///
/// [`ParseOutcome`]: super::ParseOutcome
#[derive(Debug, Error)]
pub enum Error {
    #[error("parser initialization failed for `{language}`: {detail}")]
    ParserInit { language: String, detail: String },

    #[error("unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("internal error: {0}")]
    Internal(String),
}
