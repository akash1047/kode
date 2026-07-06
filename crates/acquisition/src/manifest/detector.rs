use crate::manifest::model::ManifestKind;

pub trait ManifestDetector {
    fn detect(&self, filename: &str) -> Option<ManifestKind>;
}
