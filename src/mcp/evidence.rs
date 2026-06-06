use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct Span {
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Default)]
pub struct EvidenceCollector {
    spans: Vec<Span>,
    touched: BTreeSet<String>,
}

impl EvidenceCollector {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::default()))
    }

    pub fn record_read(&mut self, path: &str, start_line: usize, end_line: usize) {
        self.touched.insert(path.to_string());
        self.spans.push(Span {
            path: path.to_string(),
            start_line,
            end_line,
        });
    }

    pub fn record_touch(&mut self, path: &str) {
        self.touched.insert(path.to_string());
    }

    pub fn spans(&self) -> &[Span] {
        &self.spans
    }

    pub fn touched_only(&self) -> Vec<PathBuf> {
        let read_paths: BTreeSet<&String> = self.spans.iter().map(|s| &s.path).collect();
        self.touched
            .iter()
            .filter(|p| !read_paths.contains(p))
            .map(PathBuf::from)
            .collect()
    }

    pub fn merged_spans(&self) -> Vec<Span> {
        let mut by_path: std::collections::BTreeMap<String, Vec<(usize, usize)>> =
            std::collections::BTreeMap::new();
        for s in &self.spans {
            by_path
                .entry(s.path.clone())
                .or_default()
                .push((s.start_line, s.end_line));
        }
        let mut out = Vec::new();
        for (path, mut ranges) in by_path {
            ranges.sort();
            let mut merged: Vec<(usize, usize)> = Vec::new();
            for (s, e) in ranges {
                if let Some(last) = merged.last_mut()
                    && s <= last.1 + 1
                {
                    last.1 = last.1.max(e);
                } else {
                    merged.push((s, e));
                }
            }
            for (s, e) in merged {
                out.push(Span {
                    path: path.clone(),
                    start_line: s,
                    end_line: e,
                });
            }
        }
        out
    }
}

pub fn format_sources_footer(collector: &EvidenceCollector) -> String {
    let spans = collector.merged_spans();
    let touched = collector.touched_only();
    if spans.is_empty() && touched.is_empty() {
        return String::new();
    }

    let mut out = String::from("\n\n**Sources:**\n");
    for s in spans {
        if s.start_line == s.end_line {
            out.push_str(&format!("- {}:{}\n", s.path, s.start_line));
        } else {
            out.push_str(&format!(
                "- {}:{}-{}\n",
                s.path, s.start_line, s.end_line
            ));
        }
    }
    if !touched.is_empty() {
        out.push_str("\n*Also examined:* ");
        let parts: Vec<String> = touched
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        out.push_str(&parts.join(", "));
        out.push('\n');
    }
    out
}
