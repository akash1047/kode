use std::path::Path;

use super::walker::walker;

pub struct ProjectStats {
    pub total_files: usize,
    pub by_extension: Vec<(String, usize)>,
    pub top_level_dirs: Vec<(String, usize)>,
    pub manifest_files: Vec<String>,
    pub entrypoint_candidates: Vec<String>,
}

pub fn stats(root: &Path) -> ProjectStats {
    use std::collections::HashMap;

    let mut ext_counts: HashMap<String, usize> = HashMap::new();
    let mut dir_counts: HashMap<String, usize> = HashMap::new();
    let mut manifests = Vec::new();
    let mut entrypoints = Vec::new();
    let mut total = 0usize;

    let manifest_names = [
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "setup.py",
        "go.mod",
        "pom.xml",
        "build.gradle",
        "build.gradle.kts",
        "composer.json",
        "Gemfile",
        "Pipfile",
        "requirements.txt",
        "deno.json",
        "tsconfig.json",
        "CMakeLists.txt",
    ];

    let entry_names = [
        "main.rs",
        "main.go",
        "main.py",
        "main.ts",
        "main.js",
        "index.ts",
        "index.js",
        "app.py",
        "server.py",
        "__main__.py",
        "Main.java",
        "Main.kt",
    ];

    for r in walker(root) {
        let Ok(entry) = r else { continue };
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        total += 1;

        let p = entry.path();
        let rel = p.strip_prefix(root).unwrap_or(p);
        let rel_str = rel.display().to_string();

        if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
            *ext_counts.entry(ext.to_lowercase()).or_default() += 1;
        }

        if let Some(first) = rel.iter().next().and_then(|s| s.to_str()) {
            if rel.iter().count() > 1 {
                *dir_counts.entry(first.to_string()).or_default() += 1;
            }
        }

        if let Some(file_name) = p.file_name().and_then(|s| s.to_str()) {
            if manifest_names.contains(&file_name) {
                manifests.push(rel_str.clone());
            }
            if entry_names.contains(&file_name) {
                entrypoints.push(rel_str);
            }
        }
    }

    let mut by_ext: Vec<_> = ext_counts.into_iter().collect();
    by_ext.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    by_ext.truncate(15);

    let mut by_dir: Vec<_> = dir_counts.into_iter().collect();
    by_dir.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    by_dir.truncate(20);

    manifests.sort();
    entrypoints.sort();

    ProjectStats {
        total_files: total,
        by_extension: by_ext,
        top_level_dirs: by_dir,
        manifest_files: manifests,
        entrypoint_candidates: entrypoints,
    }
}
