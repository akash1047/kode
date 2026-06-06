use std::fs;
use std::path::Path;

use regex::RegexBuilder;
use serde::Serialize;

use super::walker::walker;

#[derive(Serialize)]
pub struct SearchHit {
    pub path: String,
    pub line: usize,
    pub col: usize,
    pub snippet: String,
}

pub struct SearchOpts<'a> {
    pub pattern: &'a str,
    pub path_contains: Option<&'a str>,
    pub max_hits: usize,
    pub case_insensitive: bool,
}

pub fn search(root: &Path, opts: SearchOpts<'_>) -> Result<Vec<SearchHit>, String> {
    let re = RegexBuilder::new(opts.pattern)
        .case_insensitive(opts.case_insensitive)
        .build()
        .map_err(|e| format!("invalid regex: {}", e))?;

    let mut hits = Vec::new();

    for r in walker(root) {
        if hits.len() >= opts.max_hits {
            break;
        }
        let Ok(entry) = r else { continue };
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let p = entry.path();
        let rel = p.strip_prefix(root).unwrap_or(p);
        let rel_str = rel.display().to_string();

        if let Some(filter) = opts.path_contains {
            if !rel_str.contains(filter) {
                continue;
            }
        }

        let Ok(content) = fs::read_to_string(p) else { continue };
        for (idx, line) in content.lines().enumerate() {
            if hits.len() >= opts.max_hits {
                break;
            }
            if let Some(m) = re.find(line) {
                hits.push(SearchHit {
                    path: rel_str.clone(),
                    line: idx + 1,
                    col: line[..m.start()].chars().count() + 1,
                    snippet: trim_snippet(line, 200),
                });
            }
        }
    }

    Ok(hits)
}

fn trim_snippet(line: &str, max: usize) -> String {
    let trimmed = line.trim_end();
    if trimmed.chars().count() <= max {
        return trimmed.to_string();
    }
    let head: String = trimmed.chars().take(max).collect();
    format!("{}…", head)
}
