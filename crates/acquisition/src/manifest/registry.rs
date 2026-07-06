use crate::manifest::detector::ManifestDetector;
use crate::manifest::model::ManifestKind;

pub struct ManifestRegistry {
    detectors: Vec<Box<dyn ManifestDetector>>,
}

impl ManifestRegistry {
    pub fn new() -> Self {
        Self {
            detectors: Vec::new(),
        }
    }

    pub fn register(&mut self, detector: Box<dyn ManifestDetector>) {
        self.detectors.push(detector);
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn ManifestDetector> {
        self.detectors.iter().map(|d| d.as_ref())
    }
}

impl Default for ManifestRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(crate::manifest::cargo::CargoManifestDetector));
        registry
    }
}

impl ManifestDetector for ManifestRegistry {
    fn detect(&self, filename: &str) -> Option<ManifestKind> {
        for detector in &self.detectors {
            if let Some(kind) = detector.detect(filename) {
                return Some(kind);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockManifestDetectorA;

    impl ManifestDetector for MockManifestDetectorA {
        fn detect(&self, filename: &str) -> Option<ManifestKind> {
            match filename {
                "a.toml" => Some(ManifestKind::CargoManifest),
                _ => None,
            }
        }
    }

    struct MockManifestDetectorB;

    impl ManifestDetector for MockManifestDetectorB {
        fn detect(&self, filename: &str) -> Option<ManifestKind> {
            match filename {
                "b.toml" => Some(ManifestKind::CargoManifest),
                _ => None,
            }
        }
    }

    #[test]
    fn empty_registry_returns_none() {
        let registry = ManifestRegistry::new();
        assert_eq!(registry.detect("Cargo.toml"), None);
    }

    #[test]
    fn first_matching_detector_wins() {
        let mut registry = ManifestRegistry::new();
        registry.register(Box::new(MockManifestDetectorA));
        registry.register(Box::new(MockManifestDetectorB));
        assert_eq!(
            registry.detect("a.toml"),
            Some(ManifestKind::CargoManifest)
        );
    }

    #[test]
    fn multiple_detectors_fallback() {
        let mut registry = ManifestRegistry::new();
        registry.register(Box::new(MockManifestDetectorA));
        registry.register(Box::new(MockManifestDetectorB));
        assert_eq!(
            registry.detect("b.toml"),
            Some(ManifestKind::CargoManifest)
        );
    }

    #[test]
    fn no_match_returns_none() {
        let mut registry = ManifestRegistry::new();
        registry.register(Box::new(MockManifestDetectorA));
        registry.register(Box::new(MockManifestDetectorB));
        assert_eq!(registry.detect("unknown.toml"), None);
    }
}
