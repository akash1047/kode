pub mod banner;
mod git;
mod manifest;
pub mod manifest_parse;
mod parse;
pub mod run_config;

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

/// Metadata about a project directory, derived from manifests and git state.
pub struct ProjectInfo {
    /// Project name from the primary manifest, if detected.
    pub name: Option<String>,
    /// Authors listed in the primary manifest.
    pub authors: Vec<String>,
    /// Whether a `.git` directory exists in `root`.
    pub git_init: bool,
    /// Raw `origin` remote URL from git config, if present.
    pub remote_url: Option<String>,
    /// Normalized HTTPS web URL (GitHub/GitLab), if derivable from `remote_url`.
    pub web_url: Option<String>,
    /// Canonical absolute path to the project root.
    pub abs_path: PathBuf,
}

/// Collect project metadata for the directory at `root`.
pub fn info(root: &Path) -> ProjectInfo {
    let abs_path = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let (name, authors) = manifest::detect(root);
    let git_dir = root.join(".git");
    let git_init = git_dir.exists();
    let remote_url = if git_init { git::read_remote(&git_dir) } else { None };
    let web_url = remote_url.as_deref().and_then(git::normalize_to_web);
    ProjectInfo { name, authors, git_init, remote_url, web_url, abs_path }
}

/// Serialize project metadata as a JSON `Value` for MCP tool responses.
pub fn info_json(root: &Path) -> Value {
    let i = info(root);
    json!({
        "name": i.name,
        "authors": i.authors,
        "path": i.abs_path.display().to_string(),
        "git_initialized": i.git_init,
        "in_cloud": i.remote_url.is_some(),
        "remote": i.remote_url,
        "url": i.web_url,
    })
}
