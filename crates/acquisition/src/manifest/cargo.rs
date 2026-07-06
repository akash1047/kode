use crate::manifest::detector::ManifestDetector;
use crate::manifest::model::ManifestKind;

pub struct CargoManifestDetector;

impl ManifestDetector for CargoManifestDetector {
    fn detect(&self, filename: &str) -> Option<ManifestKind> {
        match filename {
            "Cargo.toml" => Some(ManifestKind::CargoManifest),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_manifest_detected() {
        let detector = CargoManifestDetector;
        assert_eq!(
            detector.detect("Cargo.toml"),
            Some(ManifestKind::CargoManifest)
        );
    }

    #[test]
    fn unknown_file_not_detected() {
        let detector = CargoManifestDetector;
        assert_eq!(detector.detect("not_a_manifest.txt"), None);
    }
}
