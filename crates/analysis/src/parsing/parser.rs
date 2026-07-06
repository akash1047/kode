use std::fmt::Debug;
use std::sync::Arc;

use kode_acquisition::{Language, RepositoryFile};

use crate::parsing::result::ParseOutcome;

/// Language-specific parser that transforms source text into a [`SyntaxTree`].
///
/// # Contract
///
/// - [`language`](Self::language) identifies which language this parser handles.
/// - [`parse`](Self::parse) must handle any valid source text for the language;
///   invalid syntax produces `Recovered` or `Failed`, never panics.
/// - Parsers must be deterministic: same source + same file produces the same outcome.
/// - Parsers must not access the filesystem.
///
/// [`SyntaxTree`]: crate::parsing::SyntaxTree
pub trait Parser: Debug {
    fn language(&self) -> Language;

    fn parse(&self, source: Arc<str>, file: &RepositoryFile) -> ParseOutcome;
}
