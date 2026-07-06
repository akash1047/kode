use crate::manifest::model::ManifestKind;

/// Determines the manifest kind from a filename.
///
/// # Contract
///
/// - Returns `None` when the filename is not a recognized manifest.
/// - Must not access the filesystem (filename-based detection only).
/// - Should be deterministic: same filename always produces the same result.
pub trait ManifestDetector {
    fn detect(&self, filename: &str) -> Option<ManifestKind>;
}
