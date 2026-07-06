use std::path::Path;

use crate::language::detector::LanguageDetector;
use crate::language::extension::ExtensionLanguageDetector;
use crate::language::model::Language;

/// Ordered collection of language detectors with first-match dispatch.
///
/// # Registration Order
///
/// Detectors are consulted in registration order. The first detector
/// that returns `Some(language)` wins. This allows custom detectors
/// to override the default extension-based detection.
///
/// # Extension
///
/// New detection strategies implement [`LanguageDetector`] and register
/// via [`register`](Self::register). Custom detectors registered before
/// the default will take priority.
pub struct LanguageRegistry {
    detectors: Vec<Box<dyn LanguageDetector>>,
}

impl LanguageRegistry {
    pub fn new() -> Self {
        Self {
            detectors: Vec::new(),
        }
    }

    pub fn register(&mut self, detector: Box<dyn LanguageDetector>) {
        self.detectors.push(detector);
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn LanguageDetector> {
        self.detectors.iter().map(|d| d.as_ref())
    }
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(ExtensionLanguageDetector));
        registry
    }
}

impl LanguageDetector for LanguageRegistry {
    fn detect(&self, path: &Path) -> Option<Language> {
        for detector in &self.detectors {
            if let Some(lang) = detector.detect(path) {
                return Some(lang);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockCustomDetector;

    impl LanguageDetector for MockCustomDetector {
        fn detect(&self, path: &Path) -> Option<Language> {
            if path.extension()?.to_str()? == "custom" {
                Some(Language::Rust)
            } else {
                None
            }
        }
    }

    #[test]
    fn empty_registry_returns_none() {
        let registry = LanguageRegistry::new();
        assert_eq!(registry.detect(Path::new("test.rs")), None);
    }

    #[test]
    fn first_matching_detector_wins() {
        let mut registry = LanguageRegistry::new();
        registry.register(Box::new(MockCustomDetector));
        registry.register(Box::new(ExtensionLanguageDetector));
        assert_eq!(
            registry.detect(Path::new("test.custom")),
            Some(Language::Rust)
        );
    }

    #[test]
    fn fallback_to_next_detector() {
        let mut registry = LanguageRegistry::new();
        registry.register(Box::new(MockCustomDetector));
        registry.register(Box::new(ExtensionLanguageDetector));
        assert_eq!(registry.detect(Path::new("main.rs")), Some(Language::Rust));
    }

    #[test]
    fn no_match_returns_none() {
        let mut registry = LanguageRegistry::new();
        registry.register(Box::new(MockCustomDetector));
        assert_eq!(registry.detect(Path::new("test.xyz")), None);
    }

    #[test]
    fn default_registry_includes_extension_detector() {
        let registry = LanguageRegistry::default();
        assert_eq!(registry.detect(Path::new("main.rs")), Some(Language::Rust));
    }
}
