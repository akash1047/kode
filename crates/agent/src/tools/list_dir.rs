//! List directory entries under the project root.

use std::fs;
use std::path::Path;

use super::path::{is_skip_dir, relative_to_root, resolve_under_root};

/// List files and directories at `path` relative to `root`.
pub fn list_dir(root: &Path, path: &str, max_entries: usize) -> Result<String, String> {
    let dir = resolve_under_root(root, path).map_err(|e| e.to_string())?;
    if !dir.is_dir() {
        return Err(format!("not a directory: {}", relative_to_root(root, &dir)));
    }

    let mut entries: Vec<_> = fs::read_dir(&dir)
        .map_err(|e| format!("read_dir {}: {e}", dir.display()))?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut lines = Vec::new();
    let mut count = 0usize;
    let mut skipped = 0usize;

    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) && is_skip_dir(&name) {
            lines.push(format!("[dir]  {name}/  (skipped contents by default)"));
            count += 1;
            if count >= max_entries {
                break;
            }
            continue;
        }

        if count >= max_entries {
            skipped += 1;
            continue;
        }

        let ft = entry.file_type().ok();
        let tag = if ft.as_ref().is_some_and(|t| t.is_dir()) {
            "[dir] "
        } else if ft.as_ref().is_some_and(|t| t.is_symlink()) {
            "[link]"
        } else {
            "[file]"
        };
        let suffix = if ft.as_ref().is_some_and(|t| t.is_dir()) {
            "/"
        } else {
            ""
        };
        lines.push(format!("{tag} {name}{suffix}"));
        count += 1;
    }

    let header = format!(
        "list_dir {} ({} shown{})",
        relative_to_root(root, &dir),
        count,
        if skipped > 0 {
            format!(", {skipped} more truncated")
        } else {
            String::new()
        }
    );

    if lines.is_empty() {
        Ok(format!("{header}\n(empty)"))
    } else {
        Ok(format!("{header}\n{}", lines.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn lists_files() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.rs"), "fn main(){}").unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        let out = list_dir(dir.path(), ".", 50).unwrap();
        assert!(out.contains("a.rs"));
        assert!(out.contains("src/"));
    }
}
