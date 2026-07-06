use std::fmt::Debug;

use kode_acquisition::Language;

use crate::parsing::syntax::SyntaxTree;

use crate::extraction::result::ExtractionOutcome;

/// Language-specific extractor that transforms a [`SyntaxTree`] into entities.
///
/// # Contract
///
/// - [`language`](Self::language) identifies which language this extractor handles.
/// - [`extract`](Self::extract) walks the syntax tree and produces entities with
///   evidence attached. Extraction continues past recoverable issues, reporting
///   them as diagnostics.
/// - Extractors must be deterministic: the same syntax tree always produces the
///   same entities.
/// - Extractors must not access the filesystem.
pub trait Extractor: Debug {
    fn language(&self) -> Language;

    fn extract(&self, tree: &SyntaxTree) -> ExtractionOutcome;
}
