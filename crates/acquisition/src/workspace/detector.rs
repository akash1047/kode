use std::path::Path;

use crate::error::Error;
use crate::workspace::model::Workspace;

pub trait WorkspaceDetector {
    fn detect(&self, root: &Path) -> Result<Workspace, Error>;
}
