use std::path::Path;

use crate::error::Error;
use crate::workspace::model::Workspace;

/// Determines the workspace structure of a repository root.
///
/// # Contract
///
/// - Returns `WorkspaceKind::None` when the root is not a recognized workspace.
/// - Returns an error only for unrecoverable failures (e.g., I/O errors).
/// - Must be deterministic for the same filesystem state.
///
/// # Ownership
///
/// Workspace detection is part of Stage 1 (Acquisition) and occurs before
/// any file traversal or parsing.
pub trait WorkspaceDetector {
    fn detect(&self, root: &Path) -> Result<Workspace, Error>;
}
