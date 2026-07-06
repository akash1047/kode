use thiserror::Error;

/// Errors that can occur during fact extraction.
///
/// These are internal errors from the extraction subsystem. Per-entity
/// extraction failures are reported as [`ExtractionDiagnostic`] values
/// rather than errors.
///
/// [`ExtractionDiagnostic`]: super::ExtractionDiagnostic
#[derive(Debug, Error)]
pub enum Error {
    #[error("extractor initialization failed for `{language}`: {detail}")]
    ExtractorInit { language: String, detail: String },

    #[error("unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("internal error: {0}")]
    Internal(String),
}
