//! Path sandbox: resolve user paths under a project root and deny secrets.

use std::path::{Component, Path, PathBuf};

use thiserror::Error;

const DENY_NAMES: &[&str] = &[
    ".env",
    ".env.local",
    ".env.production",
    "credentials.json",
    "service-account.json",
    "id_rsa",
    "id_ed25519",
];

const DENY_SUFFIXES: &[&str] = &[".pem", ".key", ".p12", ".pfx"];

/// Errors from path resolution / sandbox checks.
#[derive(Debug, Error)]
pub enum PathError {
    #[error("project root not found: {0}")]
    RootMissing(String),
    #[error("path escapes project root: {0}")]
    Escape(String),
    #[error("cannot resolve {0}: {1}")]
    Resolve(String, String),
    #[error("access denied to sensitive file: {0}")]
    Denied(String),
}

/// Resolve `user_path` under `root`, rejecting escapes outside the root.
pub fn resolve_under_root(root: &Path, user_path: &str) -> Result<PathBuf, PathError> {
    let root = root
        .canonicalize()
        .map_err(|_| PathError::RootMissing(root.display().to_string()))?;

    let requested = if user_path.is_empty() || user_path == "." {
        root.clone()
    } else {
        let p = Path::new(user_path);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            root.join(p)
        }
    };

    let normalized = normalize_path(&requested);

    let resolved = if normalized.exists() {
        normalized
            .canonicalize()
            .map_err(|e| PathError::Resolve(normalized.display().to_string(), e.to_string()))?
    } else if let Some(parent) = normalized.parent() {
        if parent.exists() {
            let parent_c = parent
                .canonicalize()
                .map_err(|e| PathError::Resolve(parent.display().to_string(), e.to_string()))?;
            if !parent_c.starts_with(&root) {
                return Err(PathError::Escape(user_path.to_string()));
            }
            return Ok(parent_c.join(normalized.file_name().unwrap_or_default()));
        } else {
            normalized
        }
    } else {
        normalized
    };

    if !resolved.starts_with(&root) && resolved != root {
        return Err(PathError::Escape(user_path.to_string()));
    }

    Ok(resolved)
}

/// Reject denied secret-like basenames (best-effort).
pub fn check_not_denied(path: &Path) -> Result<(), PathError> {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if DENY_NAMES.iter().any(|d| name == *d) {
        return Err(PathError::Denied(name));
    }
    if DENY_SUFFIXES.iter().any(|s| name.ends_with(s)) {
        return Err(PathError::Denied(name));
    }
    Ok(())
}

/// Display path relative to root when possible.
pub fn relative_to_root(root: &Path, path: &Path) -> String {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    path.strip_prefix(&root)
        .map(|p| {
            if p.as_os_str().is_empty() {
                ".".into()
            } else {
                p.display().to_string()
            }
        })
        .unwrap_or_else(|_| path.display().to_string())
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::Prefix(p) => out.push(p.as_os_str()),
            Component::RootDir => out.push(Component::RootDir.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(c) => out.push(c),
        }
    }
    out
}

/// Heavy directories to skip while walking.
pub fn is_skip_dir(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | "target"
            | "node_modules"
            | ".svn"
            | ".hg"
            | "dist"
            | "build"
            | ".next"
            | "__pycache__"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn allows_relative_under_root() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        let p = resolve_under_root(dir.path(), "src").unwrap();
        assert!(p.ends_with("src"));
    }

    #[test]
    fn rejects_parent_escape() {
        let dir = tempdir().unwrap();
        let err = resolve_under_root(dir.path(), "../").unwrap_err();
        assert!(matches!(err, PathError::Escape(_)));
    }

    #[test]
    fn denies_env_file() {
        let dir = tempdir().unwrap();
        let p = dir.path().join(".env");
        fs::write(&p, "SECRET=1").unwrap();
        let resolved = resolve_under_root(dir.path(), ".env").unwrap();
        let err = check_not_denied(&resolved).unwrap_err();
        assert!(matches!(err, PathError::Denied(_)));
    }
}
