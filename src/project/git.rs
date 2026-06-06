use std::fs;
use std::path::Path;

use super::parse::extract_kv;

pub fn read_remote(git_dir: &Path) -> Option<String> {
    let cfg = fs::read_to_string(git_dir.join("config")).ok()?;
    let mut in_origin = false;
    for raw in cfg.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_origin = line == "[remote \"origin\"]";
            continue;
        }
        if !in_origin {
            continue;
        }
        if let Some(v) = extract_kv(line, "url") {
            return Some(v.trim().to_string());
        }
    }
    None
}

pub fn normalize_to_web(remote: &str) -> Option<String> {
    let r = remote.trim();
    if r.is_empty() {
        return None;
    }
    if let Some(rest) = r.strip_prefix("git@") {
        let (host, path) = rest.split_once(':')?;
        let path = path.trim_end_matches(".git");
        return Some(format!("https://{}/{}", host, path));
    }
    if r.starts_with("ssh://") {
        let rest = &r[6..];
        let rest = rest.strip_prefix("git@").unwrap_or(rest);
        let (host, path) = rest.split_once('/')?;
        let path = path.trim_end_matches(".git");
        return Some(format!("https://{}/{}", host, path));
    }
    if r.starts_with("https://") || r.starts_with("http://") {
        return Some(r.trim_end_matches(".git").to_string());
    }
    Some(r.to_string())
}
