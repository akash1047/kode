//! Regex search under the project root.

use std::fs;
use std::path::Path;

use regex::RegexBuilder;
use walkdir::WalkDir;

use super::path::{check_not_denied, is_skip_dir, relative_to_root, resolve_under_root};

const MAX_FILE_BYTES: u64 = 1_000_000;

/// Search file contents with a regular expression.
pub fn grep(
    root: &Path,
    pattern: &str,
    path: &str,
    glob: Option<&str>,
    case_insensitive: bool,
    max_matches: usize,
) -> Result<String, String> {
    if pattern.is_empty() {
        return Err("pattern must not be empty".into());
    }

    let re = RegexBuilder::new(pattern)
        .case_insensitive(case_insensitive)
        .build()
        .map_err(|e| format!("invalid regex: {e}"))?;

    let start = resolve_under_root(root, path).map_err(|e| e.to_string())?;
    let mut matches: Vec<String> = Vec::new();
    let mut files_scanned = 0usize;

    if start.is_file() {
        files_scanned = 1;
        if let Some(lines) = search_file(root, &start, &re, max_matches) {
            matches.extend(lines);
        }
        return Ok(format_result(pattern, &matches, files_scanned, max_matches));
    }

    let walker = WalkDir::new(&start).into_iter().filter_entry(|e| {
        if e.file_type().is_dir() {
            let name = e.file_name().to_string_lossy();
            !is_skip_dir(&name)
        } else {
            true
        }
    });

    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let p = entry.path();
        if check_not_denied(p).is_err() {
            continue;
        }
        if let Some(g) = glob {
            if !glob_match(g, p) {
                continue;
            }
        }
        files_scanned += 1;
        let budget = max_matches.saturating_sub(matches.len());
        if let Some(lines) = search_file(root, p, &re, budget) {
            matches.extend(lines);
        }
        if matches.len() >= max_matches {
            break;
        }
    }

    Ok(format_result(pattern, &matches, files_scanned, max_matches))
}

fn search_file(root: &Path, path: &Path, re: &regex::Regex, budget: usize) -> Option<Vec<String>> {
    if budget == 0 {
        return None;
    }
    let meta = fs::metadata(path).ok()?;
    if meta.len() > MAX_FILE_BYTES {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    if bytes.contains(&0) {
        return None;
    }
    let text = String::from_utf8_lossy(&bytes);
    let rel = relative_to_root(root, path);
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if re.is_match(line) {
            let trimmed = if line.len() > 300 {
                format!("{}…", &line[..300])
            } else {
                line.to_string()
            };
            out.push(format!("{rel}:{}:{trimmed}", i + 1));
            if out.len() >= budget {
                break;
            }
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn format_result(pattern: &str, matches: &[String], files_scanned: usize, max: usize) -> String {
    if matches.is_empty() {
        return format!("grep {pattern:?}: no matches ({files_scanned} files scanned)");
    }
    let truncated = matches.len() >= max;
    format!(
        "grep {pattern:?}: {} match(es) in {files_scanned} files{}\n{}",
        matches.len(),
        if truncated { " (truncated)" } else { "" },
        matches.join("\n")
    )
}

fn glob_match(pattern: &str, path: &Path) -> bool {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if let Some(suf) = pattern.strip_prefix('*') {
        return name.ends_with(suf);
    }
    if let Some(pre) = pattern.strip_suffix('*') {
        return name.starts_with(pre);
    }
    name == pattern || path.to_string_lossy().contains(pattern)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn finds_pattern() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("a.rs"),
            "fn chat_stream() {}\nfn other() {}\n",
        )
        .unwrap();
        let out = grep(dir.path(), "chat_stream", ".", None, false, 20).unwrap();
        assert!(out.contains("chat_stream"));
        assert!(out.contains("a.rs"));
    }

    #[test]
    fn no_match() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.rs"), "hello\n").unwrap();
        let out = grep(dir.path(), "zzz_not_found", ".", None, false, 20).unwrap();
        assert!(out.contains("no matches"));
    }
}
