//! Read text files under the project root with line numbers.

use std::fs;
use std::path::Path;

use super::path::{check_not_denied, relative_to_root, resolve_under_root};

const MAX_BYTES: usize = 256_000;

/// Read a file with optional 1-based offset and line limit.
pub fn read_file(root: &Path, path: &str, offset: usize, limit: usize) -> Result<String, String> {
    let offset = offset.max(1);
    let limit = limit.clamp(1, 2000);

    let file = resolve_under_root(root, path).map_err(|e| e.to_string())?;
    check_not_denied(&file).map_err(|e| e.to_string())?;
    if !file.is_file() {
        return Err(format!("not a file: {}", relative_to_root(root, &file)));
    }

    let bytes = fs::read(&file).map_err(|e| format!("read {}: {e}", file.display()))?;
    if bytes.contains(&0) {
        return Err(format!(
            "binary file refused: {}",
            relative_to_root(root, &file)
        ));
    }

    let text = String::from_utf8_lossy(&bytes);
    let text = if text.len() > MAX_BYTES {
        &text[..MAX_BYTES]
    } else {
        &text
    };

    let all_lines: Vec<&str> = text.lines().collect();
    let total = all_lines.len();
    let start = offset.saturating_sub(1).min(total);
    let end = (start + limit).min(total);
    let slice = &all_lines[start..end];

    let mut body = String::new();
    for (i, line) in slice.iter().enumerate() {
        let n = start + i + 1;
        let line = if line.len() > 500 {
            format!("{}…", &line[..500])
        } else {
            (*line).to_string()
        };
        body.push_str(&format!("{n:>6}|{line}\n"));
    }

    let start_display = if total == 0 { 0 } else { start + 1 };
    let trunc = if text.len() >= MAX_BYTES {
        " (file truncated to byte cap)"
    } else {
        ""
    };
    let body = if body.is_empty() {
        "(empty)\n".to_string()
    } else {
        body
    };

    Ok(format!(
        "read_file {} lines {start_display}-{end} of {total}{trunc}\n{body}",
        relative_to_root(root, &file),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn reads_with_line_numbers() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.rs"), "one\ntwo\nthree\n").unwrap();
        let out = read_file(dir.path(), "a.rs", 1, 10).unwrap();
        assert!(out.contains("1|one"));
        assert!(out.contains("2|two"));
    }

    #[test]
    fn offset_and_limit() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.rs"), "a\nb\nc\nd\n").unwrap();
        let out = read_file(dir.path(), "a.rs", 2, 2).unwrap();
        assert!(out.contains("2|b"));
        assert!(out.contains("3|c"));
        assert!(!out.contains("1|a"));
    }
}
