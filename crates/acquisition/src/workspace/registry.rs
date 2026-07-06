use std::path::Path;

use crate::error::Error;
use crate::workspace::detector::WorkspaceDetector;
use crate::workspace::model::{Workspace, WorkspaceKind};

/// Ordered collection of workspace detectors with first-match dispatch.
///
/// Unlike other registries, workspace detectors that return
/// [`WorkspaceKind::None`] are skipped to allow fallthrough to the
/// next detector. Only when all detectors return `None` does the
/// registry return `WorkspaceKind::None`.
///
/// # Extension
///
/// New workspace types require a [`WorkspaceDetector`] implementation
/// and registration via [`register`](Self::register).
pub struct WorkspaceRegistry {
    detectors: Vec<Box<dyn WorkspaceDetector>>,
}

impl WorkspaceRegistry {
    pub fn new() -> Self {
        Self {
            detectors: Vec::new(),
        }
    }

    pub fn register(&mut self, detector: Box<dyn WorkspaceDetector>) {
        self.detectors.push(detector);
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn WorkspaceDetector> {
        self.detectors.iter().map(|d| d.as_ref())
    }
}

impl Default for WorkspaceRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(crate::workspace::cargo::CargoWorkspaceDetector));
        registry
    }
}

impl WorkspaceDetector for WorkspaceRegistry {
    fn detect(&self, root: &Path) -> Result<Workspace, Error> {
        for detector in &self.detectors {
            let workspace = detector.detect(root)?;
            if !matches!(workspace.kind(), WorkspaceKind::None) {
                return Ok(workspace);
            }
        }
        Ok(Workspace {
            kind: WorkspaceKind::None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct MockWorkspaceDetectorA;

    impl WorkspaceDetector for MockWorkspaceDetectorA {
        fn detect(&self, _root: &Path) -> Result<Workspace, Error> {
            Ok(Workspace {
                kind: WorkspaceKind::SinglePackage {
                    manifest: PathBuf::from("a/Cargo.toml"),
                },
            })
        }
    }

    struct MockWorkspaceDetectorNone;

    impl WorkspaceDetector for MockWorkspaceDetectorNone {
        fn detect(&self, _root: &Path) -> Result<Workspace, Error> {
            Ok(Workspace {
                kind: WorkspaceKind::None,
            })
        }
    }

    #[test]
    fn empty_registry_returns_none() {
        let registry = WorkspaceRegistry::new();
        let dir = tempfile::TempDir::new().unwrap();
        let ws = registry.detect(dir.path()).unwrap();
        assert_eq!(ws.kind(), &WorkspaceKind::None);
    }

    #[test]
    fn first_matching_detector_wins() {
        let mut registry = WorkspaceRegistry::new();
        registry.register(Box::new(MockWorkspaceDetectorNone));
        registry.register(Box::new(MockWorkspaceDetectorA));
        let dir = tempfile::TempDir::new().unwrap();
        let ws = registry.detect(dir.path()).unwrap();
        assert_eq!(
            ws.kind(),
            &WorkspaceKind::SinglePackage {
                manifest: PathBuf::from("a/Cargo.toml")
            }
        );
    }

    #[test]
    fn fallback_returns_none() {
        let mut registry = WorkspaceRegistry::new();
        registry.register(Box::new(MockWorkspaceDetectorNone));
        let dir = tempfile::TempDir::new().unwrap();
        let ws = registry.detect(dir.path()).unwrap();
        assert_eq!(ws.kind(), &WorkspaceKind::None);
    }
}
