use std::path::Path;

use crate::language::model::Language;

/// Determines the programming language of a file from its path.
///
/// # Contract
///
/// - Returns `None` when the language cannot be determined.
/// - Must not access the filesystem (path-based detection only).
/// - Should be deterministic: same path always produces the same result.
pub trait LanguageDetector {
    fn detect(&self, path: &Path) -> Option<Language>;
}
