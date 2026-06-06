use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use rusqlite::params;

use crate::fs::walker;
use crate::project::{manifest_parse, run_config};

use super::{Cache, hash, now_secs, symbols};

const README_HEAD_LINES: usize = 80;
const MAX_READABLE_SIZE: u64 = 256 * 1024;

#[derive(Debug, Default)]
pub struct RefreshReport {
    pub scanned: usize,
    pub added: usize,
    pub changed: usize,
    pub unchanged: usize,
    pub deleted: usize,
    pub manifests: usize,
    pub run_configs: usize,
    pub readmes: usize,
    pub symbols_files: usize,
    pub symbols_total: usize,
    pub dirs_hashed: usize,
}

pub fn refresh(cache: &mut Cache) -> Result<RefreshReport> {
    let root = cache.root.clone();
    let mut report = RefreshReport::default();

    let mut current: HashMap<String, (i64, i64)> = HashMap::new();
    let mut entries: Vec<(String, PathBuf, i64, i64)> = Vec::new();

    for r in walker(&root) {
        let Ok(entry) = r else { continue };
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let abs = entry.path();
        let rel = abs.strip_prefix(&root).unwrap_or(abs);
        let rel_str = rel.display().to_string();

        let meta = match fs::metadata(abs) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let size = meta.len() as i64;
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        current.insert(rel_str.clone(), (mtime, size));
        entries.push((rel_str, abs.to_path_buf(), mtime, size));
        report.scanned += 1;
    }

    let cached = load_cached_index(cache)?;
    let mut hashes_by_path: HashMap<String, Vec<u8>> = HashMap::new();

    let tx = cache.conn_mut().transaction()?;

    for (rel, abs, mtime, size) in entries.iter() {
        let prev = cached.get(rel);
        let (need_hash, prev_hash) = match prev {
            Some((cached_mtime, cached_size, cached_hash)) => {
                if *cached_mtime == *mtime && *cached_size == *size {
                    (false, Some(cached_hash.clone()))
                } else {
                    (true, Some(cached_hash.clone()))
                }
            }
            None => (true, None),
        };

        let new_hash = if need_hash {
            match hash::file_hash(abs) {
                Ok(h) => h.to_vec(),
                Err(_) => continue,
            }
        } else {
            prev_hash.clone().unwrap_or_default()
        };

        let changed = match &prev_hash {
            Some(h) => h != &new_hash,
            None => true,
        };

        let lang = detect_lang(abs);
        let is_binary = looks_binary(abs, *size);

        tx.execute(
            "INSERT INTO files (path, hash, mtime, size, lang, is_binary, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(path) DO UPDATE SET
                hash = excluded.hash,
                mtime = excluded.mtime,
                size = excluded.size,
                lang = excluded.lang,
                is_binary = excluded.is_binary,
                updated_at = excluded.updated_at",
            params![
                rel,
                &new_hash,
                mtime,
                size,
                lang,
                is_binary as i64,
                now_secs(),
            ],
        )?;
        hashes_by_path.insert(rel.clone(), new_hash.clone());

        if prev.is_none() {
            report.added += 1;
        } else if changed {
            report.changed += 1;
        } else {
            report.unchanged += 1;
        }

        if !changed && prev_hash.is_some() {
            continue;
        }

        if *size > MAX_READABLE_SIZE as i64 || is_binary {
            tx.execute("DELETE FROM manifests WHERE path = ?1", params![rel])?;
            tx.execute("DELETE FROM run_configs WHERE path = ?1", params![rel])?;
            tx.execute("DELETE FROM readmes WHERE path = ?1", params![rel])?;
            tx.execute("DELETE FROM symbols WHERE path = ?1", params![rel])?;
            continue;
        }

        let file_name = abs
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();

        let content = match fs::read_to_string(abs) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if manifest_parse::is_manifest(file_name) {
            match manifest_parse::parse(abs, &content) {
                Ok(parsed) => {
                    let json = serde_json::to_string(&parsed).unwrap_or_default();
                    tx.execute(
                        "INSERT INTO manifests (path, parsed_json, hash) VALUES (?1, ?2, ?3)
                         ON CONFLICT(path) DO UPDATE SET parsed_json = excluded.parsed_json, hash = excluded.hash",
                        params![rel, json, &new_hash],
                    )?;
                    report.manifests += 1;
                }
                Err(_) => {
                    tx.execute("DELETE FROM manifests WHERE path = ?1", params![rel])?;
                }
            }
        } else {
            tx.execute("DELETE FROM manifests WHERE path = ?1", params![rel])?;
        }

        if let Some(kind) = run_config::detect_kind(file_name) {
            match run_config::parse(kind, &content) {
                Ok(parsed) => {
                    let json = serde_json::to_string(&parsed).unwrap_or_default();
                    tx.execute(
                        "INSERT INTO run_configs (path, kind, parsed_json, hash) VALUES (?1, ?2, ?3, ?4)
                         ON CONFLICT(path) DO UPDATE SET kind = excluded.kind, parsed_json = excluded.parsed_json, hash = excluded.hash",
                        params![rel, kind, json, &new_hash],
                    )?;
                    report.run_configs += 1;
                }
                Err(_) => {
                    tx.execute("DELETE FROM run_configs WHERE path = ?1", params![rel])?;
                }
            }
        } else {
            tx.execute("DELETE FROM run_configs WHERE path = ?1", params![rel])?;
        }

        if is_readme(file_name) {
            let (head, total) = readme_head(&content, README_HEAD_LINES);
            tx.execute(
                "INSERT INTO readmes (path, text_head, line_count, hash) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(path) DO UPDATE SET text_head = excluded.text_head, line_count = excluded.line_count, hash = excluded.hash",
                params![rel, head, total as i64, &new_hash],
            )?;
            report.readmes += 1;
        } else {
            tx.execute("DELETE FROM readmes WHERE path = ?1", params![rel])?;
        }

        tx.execute("DELETE FROM symbols WHERE path = ?1", params![rel])?;
        if let Some(l) = lang.filter(|l| symbols::is_supported(l)) {
            if let Ok(syms) = symbols::extract(l, &content) {
                if !syms.is_empty() {
                    {
                        let mut stmt = tx.prepare_cached(
                            "INSERT INTO symbols (path, name, kind, start_line, end_line, signature)
                             VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
                        )?;
                        for s in &syms {
                            stmt.execute(params![
                                rel,
                                &s.name,
                                &s.kind,
                                s.start_line as i64,
                                s.end_line as i64,
                            ])?;
                        }
                    }
                    report.symbols_files += 1;
                    report.symbols_total += syms.len();
                }
            }
        }
    }

    for (rel, _) in cached.iter() {
        if !current.contains_key(rel) {
            tx.execute("DELETE FROM files WHERE path = ?1", params![rel])?;
            report.deleted += 1;
        }
    }

    let dir_hashes = compute_dir_hashes(&hashes_by_path);
    tx.execute("DELETE FROM dir_hashes", [])?;
    {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO dir_hashes (dir_path, child_hash, file_count, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
        )?;
        let now = now_secs();
        for (dir, h, count) in &dir_hashes {
            stmt.execute(params![dir, &h.to_vec(), *count as i64, now])?;
        }
        report.dirs_hashed = dir_hashes.len();
    }

    tx.commit()?;

    cache.mark_scanned()?;
    Ok(report)
}

fn load_cached_index(cache: &Cache) -> Result<HashMap<String, (i64, i64, Vec<u8>)>> {
    let mut stmt = cache
        .conn()
        .prepare("SELECT path, mtime, size, hash FROM files")?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, Vec<u8>>(3)?,
        ))
    })?;
    let mut out = HashMap::new();
    for row in rows {
        let (p, m, s, h) = row?;
        out.insert(p, (m, s, h));
    }
    Ok(out)
}

fn is_readme(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();
    lower == "readme"
        || lower == "readme.md"
        || lower == "readme.rst"
        || lower == "readme.txt"
        || lower == "readme.markdown"
}

fn readme_head(content: &str, max_lines: usize) -> (String, usize) {
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    let head_lines = lines.iter().take(max_lines).copied().collect::<Vec<_>>();
    (head_lines.join("\n"), total)
}

fn detect_lang(path: &Path) -> Option<&'static str> {
    let ext = path.extension().and_then(|e| e.to_str())?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "rs" => "rust",
        "py" => "python",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "go" => "go",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "rb" => "ruby",
        "php" => "php",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" => "cpp",
        "cs" => "csharp",
        "swift" => "swift",
        "md" | "markdown" => "markdown",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "json" => "json",
        "sh" | "bash" => "shell",
        _ => return None,
    })
}

fn looks_binary(path: &Path, size: i64) -> bool {
    if size == 0 {
        return false;
    }
    let sample_size = (size as usize).min(8192);
    let Ok(bytes) = read_sample(path, sample_size) else {
        return true;
    };
    bytes.iter().any(|&b| b == 0)
}

fn compute_dir_hashes(
    hashes_by_path: &HashMap<String, Vec<u8>>,
) -> Vec<(String, [u8; 8], usize)> {
    let mut file_hashes_by_dir: HashMap<String, Vec<(String, Vec<u8>)>> = HashMap::new();
    for (rel, h) in hashes_by_path {
        let dir = parent_dir(rel);
        file_hashes_by_dir
            .entry(dir)
            .or_default()
            .push((rel.clone(), h.clone()));
    }

    let mut all_dirs: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for dir in file_hashes_by_dir.keys() {
        let mut cur = dir.clone();
        loop {
            all_dirs.insert(cur.clone());
            if cur.is_empty() {
                break;
            }
            cur = parent_of_dir(&cur);
        }
    }

    let dirs_desc: Vec<String> = all_dirs.iter().rev().cloned().collect();
    let mut hashes: HashMap<String, [u8; 8]> = HashMap::new();
    let mut counts: HashMap<String, usize> = HashMap::new();

    for dir in dirs_desc {
        let mut acc: Vec<u8> = Vec::new();
        let mut count = 0usize;

        if let Some(files_here) = file_hashes_by_dir.get(&dir) {
            let mut files_sorted = files_here.clone();
            files_sorted.sort_by(|a, b| a.0.cmp(&b.0));
            for (p, h) in &files_sorted {
                acc.extend_from_slice(p.as_bytes());
                acc.push(0);
                acc.extend_from_slice(h);
                acc.push(0);
            }
            count += files_sorted.len();
        }

        let prefix = if dir.is_empty() {
            String::new()
        } else {
            format!("{dir}/")
        };
        let child_dirs: Vec<&String> = all_dirs
            .iter()
            .filter(|d| {
                if d.as_str() == dir.as_str() {
                    return false;
                }
                if dir.is_empty() {
                    !d.contains('/')
                } else {
                    d.starts_with(&prefix) && !d[prefix.len()..].contains('/')
                }
            })
            .collect();
        for cd in &child_dirs {
            if let Some(ch) = hashes.get(*cd) {
                acc.extend_from_slice(cd.as_bytes());
                acc.push(0);
                acc.extend_from_slice(ch);
                acc.push(0);
                if let Some(c) = counts.get(*cd) {
                    count += c;
                }
            }
        }

        let merkle = hash::bytes_hash(&acc);
        hashes.insert(dir.clone(), merkle);
        counts.insert(dir, count);
    }

    let mut out: Vec<(String, [u8; 8], usize)> = hashes
        .into_iter()
        .map(|(d, h)| {
            let c = counts.get(&d).copied().unwrap_or(0);
            (d, h, c)
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn parent_dir(rel: &str) -> String {
    match rel.rfind('/') {
        Some(i) => rel[..i].to_string(),
        None => String::new(),
    }
}

fn parent_of_dir(dir: &str) -> String {
    match dir.rfind('/') {
        Some(i) => dir[..i].to_string(),
        None => String::new(),
    }
}

fn read_sample(path: &Path, n: usize) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut f = fs::File::open(path)?;
    let mut buf = vec![0u8; n];
    let read = f.read(&mut buf)?;
    buf.truncate(read);
    Ok(buf)
}
