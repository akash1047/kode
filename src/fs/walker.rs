use std::path::Path;

use ignore::WalkBuilder;

pub fn walker(root: &Path) -> ignore::Walk {
    WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .filter_entry(|e| e.file_name() != ".git")
        .build()
}

pub fn list(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for r in walker(root) {
        let Ok(entry) = r else { continue };
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let p = entry.path();
        let rel = p.strip_prefix(root).unwrap_or(p);
        out.push(rel.display().to_string());
    }
    out
}
