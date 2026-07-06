use kode_acquisition::Language;

use crate::extraction::extractor::Extractor;

/// Registry of language-specific extractors with language-keyed dispatch.
///
/// # Dispatch
///
/// [`dispatch`](Self::dispatch) looks up an extractor by language with a
/// linear scan over registered extractors. Each language maps to at most
/// one extractor (first registration wins).
///
/// # Extension
///
/// New extractors implement [`Extractor`] and register via
/// [`register`](Self::register). The default registry includes the Rust
/// extractor only.
///
/// # Invariants
///
/// - Only one extractor per language is expected.
/// - Unrecognized languages return `None`.
#[derive(Debug)]
pub struct ExtractorRegistry {
    extractors: Vec<Box<dyn Extractor>>,
}

impl ExtractorRegistry {
    pub fn new() -> Self {
        Self {
            extractors: Vec::new(),
        }
    }

    pub fn register(&mut self, extractor: Box<dyn Extractor>) {
        self.extractors.push(extractor);
    }

    pub fn dispatch(&self, language: &Language) -> Option<&dyn Extractor> {
        self.extractors
            .iter()
            .find(|e| e.language() == *language)
            .map(|e| e.as_ref())
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn Extractor> {
        self.extractors.iter().map(|e| e.as_ref())
    }
}

impl Default for ExtractorRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(crate::extraction::rust::RustExtractor));
        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct MockRustExtractor;

    impl Extractor for MockRustExtractor {
        fn language(&self) -> Language {
            Language::Rust
        }

        fn extract(
            &self,
            _tree: &crate::parsing::syntax::SyntaxTree,
        ) -> crate::extraction::result::ExtractionOutcome {
            crate::extraction::result::ExtractionOutcome::Success(Vec::new())
        }
    }

    #[derive(Debug)]
    struct MockGoExtractor;

    impl Extractor for MockGoExtractor {
        fn language(&self) -> Language {
            Language::Go
        }

        fn extract(
            &self,
            _tree: &crate::parsing::syntax::SyntaxTree,
        ) -> crate::extraction::result::ExtractionOutcome {
            crate::extraction::result::ExtractionOutcome::Success(Vec::new())
        }
    }

    #[test]
    fn empty_registry_returns_no_extractor() {
        let registry = ExtractorRegistry::new();
        assert!(registry.dispatch(&Language::Rust).is_none());
    }

    #[test]
    fn dispatch_finds_extractor_by_language() {
        let mut registry = ExtractorRegistry::new();
        registry.register(Box::new(MockRustExtractor));
        registry.register(Box::new(MockGoExtractor));
        assert!(registry.dispatch(&Language::Rust).is_some());
        assert!(registry.dispatch(&Language::Go).is_some());
        assert!(registry.dispatch(&Language::Python).is_none());
    }

    #[test]
    fn first_registered_wins() {
        let mut registry = ExtractorRegistry::new();
        registry.register(Box::new(MockRustExtractor));
        let extractor = registry.dispatch(&Language::Rust).unwrap();
        assert_eq!(extractor.language(), Language::Rust);
    }

    #[test]
    fn default_registry_includes_rust_extractor() {
        let registry = ExtractorRegistry::default();
        assert!(registry.dispatch(&Language::Rust).is_some());
        assert!(registry.dispatch(&Language::Go).is_none());
    }

    #[test]
    fn registry_iter() {
        let mut registry = ExtractorRegistry::new();
        registry.register(Box::new(MockRustExtractor));
        registry.register(Box::new(MockGoExtractor));
        assert_eq!(registry.iter().count(), 2);
    }
}
