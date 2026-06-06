use std::fs;
use std::path::Path;

pub struct ReadSpan {
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub total_lines: usize,
    pub content: String,
}

pub fn read_span(
    root: &Path,
    rel: &str,
    start_line: Option<usize>,
    end_line: Option<usize>,
) -> Result<ReadSpan, String> {
    let raw = read_full(root, rel)?;
    let lines: Vec<&str> = raw.lines().collect();
    let total_lines = lines.len();

    let start = start_line.unwrap_or(1).max(1);
    let end = end_line.unwrap_or(total_lines).min(total_lines.max(1));

    if start > end {
        return Err(format!(
            "start_line ({}) greater than end_line ({})",
            start, end
        ));
    }
    if start > total_lines && total_lines > 0 {
        return Err(format!(
            "start_line ({}) exceeds file length ({} lines)",
            start, total_lines
        ));
    }

    let slice_start = start.saturating_sub(1);
    let slice_end = end.min(total_lines);
    let content = if total_lines == 0 {
        String::new()
    } else {
        lines[slice_start..slice_end].join("\n")
    };

    Ok(ReadSpan {
        path: rel.to_string(),
        start_line: start,
        end_line: slice_end,
        total_lines,
        content,
    })
}

fn read_full(root: &Path, rel: &str) -> Result<String, String> {
    if rel.contains("..") {
        return Err("path traversal not allowed".to_string());
    }
    let p = root.join(rel);
    let abs_root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let abs_p = fs::canonicalize(&p).map_err(|e| e.to_string())?;
    if !abs_p.starts_with(&abs_root) {
        return Err("path escapes project root".to_string());
    }
    fs::read_to_string(&abs_p).map_err(|e| e.to_string())
}
