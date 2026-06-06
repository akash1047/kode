use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};

use crate::project;
use crate::project::manifest_parse::{Ecosystem, ParsedManifest, WorkspaceMarker};
use crate::project::run_config::{DirectiveLine, ParsedRunConfig};

use super::Cache;

const TREE_MAX_DEPTH: usize = 3;
const TREE_NODE_CAP: usize = 25;

pub fn project_overview(cache: &Cache) -> Result<Value> {
    let root = cache.root.clone();
    let info = project::info(&root);

    let files = load_files(cache)?;
    let manifests = load_manifests(cache)?;
    let run_configs = load_run_configs(cache)?;
    let readmes = load_readmes(cache)?;

    let total_files = files.len();
    let by_extension = top_extensions(&files, 15);
    let top_level_dirs = top_level_dir_counts(&files, 20);
    let tree = build_tree(&files, TREE_MAX_DEPTH, TREE_NODE_CAP);

    let workspace = detect_workspace(&manifests);
    let applications = build_applications(&manifests, &run_configs, &readmes);
    let global_run_configs = root_level_run_configs(&run_configs);
    let root_readme = readmes.iter().find(|(p, _)| !p.contains('/')).map(|(p, r)| {
        json!({
            "path": p,
            "line_count": r.line_count,
            "head": r.text_head,
        })
    });

    Ok(json!({
        "project": {
            "name": info.name,
            "authors": info.authors,
            "abs_path": info.abs_path.display().to_string(),
            "git": {
                "initialized": info.git_init,
                "remote": info.remote_url,
                "web_url": info.web_url,
            },
            "readme": root_readme,
        },
        "structure": {
            "total_files": total_files,
            "by_extension_top15": by_extension,
            "top_level_dirs_top20": top_level_dirs,
            "tree": tree,
        },
        "workspace": workspace,
        "applications": applications,
        "global_run_configs": global_run_configs,
        "last_scan_at": cache.last_scan_at()?,
        "note": "AUTHORITATIVE snapshot of project structure from local cache. Manifests, run-configs, READMEs already parsed — do NOT call read_project_files on them again. Cite facts back to source paths (and line numbers where shown). Use read_project_files only for code-level questions about files NOT pre-parsed here."
    }))
}

#[derive(Debug)]
#[allow(dead_code)]
struct FileRow {
    path: String,
    size: i64,
    lang: Option<String>,
    is_binary: bool,
}

#[derive(Debug)]
struct ReadmeRow {
    text_head: String,
    line_count: i64,
}

fn load_files(cache: &Cache) -> Result<Vec<FileRow>> {
    let mut stmt = cache
        .conn()
        .prepare("SELECT path, size, lang, is_binary FROM files ORDER BY path")?;
    let rows = stmt.query_map([], |r| {
        Ok(FileRow {
            path: r.get(0)?,
            size: r.get(1)?,
            lang: r.get(2)?,
            is_binary: r.get::<_, i64>(3)? != 0,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

fn load_manifests(cache: &Cache) -> Result<Vec<(String, ParsedManifest)>> {
    let mut stmt = cache
        .conn()
        .prepare("SELECT path, parsed_json FROM manifests ORDER BY path")?;
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })?;
    let mut out = Vec::new();
    for r in rows {
        let (p, j) = r?;
        if let Ok(parsed) = serde_json::from_str::<ParsedManifest>(&j) {
            out.push((p, parsed));
        }
    }
    Ok(out)
}

fn load_run_configs(cache: &Cache) -> Result<Vec<(String, String, ParsedRunConfig)>> {
    let mut stmt = cache
        .conn()
        .prepare("SELECT path, kind, parsed_json FROM run_configs ORDER BY path")?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })?;
    let mut out = Vec::new();
    for r in rows {
        let (p, k, j) = r?;
        if let Ok(parsed) = serde_json::from_str::<ParsedRunConfig>(&j) {
            out.push((p, k, parsed));
        }
    }
    Ok(out)
}

fn load_readmes(cache: &Cache) -> Result<Vec<(String, ReadmeRow)>> {
    let mut stmt = cache
        .conn()
        .prepare("SELECT path, text_head, line_count FROM readmes ORDER BY path")?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            ReadmeRow {
                text_head: r.get(1)?,
                line_count: r.get(2)?,
            },
        ))
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

fn top_extensions(files: &[FileRow], cap: usize) -> Value {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for f in files {
        let path = Path::new(&f.path);
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            *counts.entry(ext.to_ascii_lowercase()).or_default() += 1;
        }
    }
    let mut v: Vec<_> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v.truncate(cap);
    Value::Object(v.into_iter().map(|(k, n)| (k, Value::from(n))).collect())
}

fn top_level_dir_counts(files: &[FileRow], cap: usize) -> Value {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for f in files {
        let path = Path::new(&f.path);
        let mut parts = path.components();
        let first = parts.next().and_then(|c| c.as_os_str().to_str());
        let has_more = parts.next().is_some();
        if let Some(first) = first {
            if has_more {
                *counts.entry(first.to_string()).or_default() += 1;
            }
        }
    }
    let mut v: Vec<_> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v.truncate(cap);
    Value::Object(v.into_iter().map(|(k, n)| (k, Value::from(n))).collect())
}

#[derive(Debug)]
#[allow(dead_code)]
struct TreeNode {
    name: String,
    is_dir: bool,
    files: usize,
    children: BTreeMap<String, TreeNode>,
}

impl TreeNode {
    fn new(name: String, is_dir: bool) -> Self {
        Self { name, is_dir, files: 0, children: BTreeMap::new() }
    }

    fn to_json(&self, path: &str, depth: usize, max_depth: usize, cap: usize) -> Value {
        if self.is_dir {
            let mut kids: Vec<(&String, &TreeNode)> = self.children.iter().collect();
            kids.sort_by(|a, b| b.1.files.cmp(&a.1.files).then(a.0.cmp(b.0)));
            let (visible, hidden) = if kids.len() > cap {
                (&kids[..cap], kids.len() - cap)
            } else {
                (&kids[..], 0)
            };
            let children_json: Vec<Value> = if depth >= max_depth {
                Vec::new()
            } else {
                visible
                    .iter()
                    .map(|(name, node)| {
                        let child_path = if path.is_empty() {
                            (*name).clone()
                        } else {
                            format!("{path}/{name}")
                        };
                        node.to_json(&child_path, depth + 1, max_depth, cap)
                    })
                    .collect()
            };
            let mut obj = serde_json::Map::new();
            obj.insert("path".into(), Value::String(path.to_string()));
            obj.insert("kind".into(), Value::String("dir".into()));
            obj.insert("files".into(), Value::from(self.files));
            if !children_json.is_empty() {
                obj.insert("children".into(), Value::Array(children_json));
            }
            if hidden > 0 {
                obj.insert("more_children".into(), Value::from(hidden));
            }
            Value::Object(obj)
        } else {
            json!({ "path": path, "kind": "file" })
        }
    }
}

fn build_tree(files: &[FileRow], max_depth: usize, cap: usize) -> Value {
    let mut root = TreeNode::new(String::new(), true);
    for f in files {
        let parts: Vec<&str> = f.path.split('/').collect();
        let mut node = &mut root;
        node.files += 1;
        for (i, part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;
            let is_dir = !is_last;
            let child = node
                .children
                .entry(part.to_string())
                .or_insert_with(|| TreeNode::new(part.to_string(), is_dir));
            if is_dir {
                child.files += 1;
            }
            node = child;
        }
    }

    let mut top: Vec<(&String, &TreeNode)> = root.children.iter().collect();
    top.sort_by(|a, b| b.1.files.cmp(&a.1.files).then(a.0.cmp(b.0)));
    let (visible, hidden) = if top.len() > cap {
        (&top[..cap], top.len() - cap)
    } else {
        (&top[..], 0)
    };

    let children: Vec<Value> = visible
        .iter()
        .map(|(name, node)| node.to_json(name, 1, max_depth, cap))
        .collect();

    let mut obj = serde_json::Map::new();
    obj.insert("path".into(), Value::String(".".into()));
    obj.insert("kind".into(), Value::String("dir".into()));
    obj.insert("files".into(), Value::from(root.files));
    obj.insert("children".into(), Value::Array(children));
    if hidden > 0 {
        obj.insert("more_children".into(), Value::from(hidden));
    }
    Value::Object(obj)
}

fn detect_workspace(manifests: &[(String, ParsedManifest)]) -> Value {
    let markers: Vec<(String, &WorkspaceMarker)> = manifests
        .iter()
        .filter_map(|(p, m)| m.workspace_marker.as_ref().map(|w| (p.clone(), w)))
        .collect();

    if !markers.is_empty() {
        let kind = match markers[0].1 {
            WorkspaceMarker::CargoWorkspace { .. } => "cargo",
            WorkspaceMarker::NpmWorkspaces { .. } => "npm/yarn/pnpm",
            WorkspaceMarker::UvWorkspace { .. } => "uv",
            WorkspaceMarker::PoetryWorkspace { .. } => "poetry",
        };
        let members: Vec<String> = match markers[0].1 {
            WorkspaceMarker::CargoWorkspace { members } => members.clone(),
            WorkspaceMarker::NpmWorkspaces { packages } => packages.clone(),
            WorkspaceMarker::UvWorkspace { members } => members.clone(),
            WorkspaceMarker::PoetryWorkspace { members } => members.clone(),
        };
        return json!({
            "is_monorepo": true,
            "reason": format!("{kind} workspace marker in {}", markers[0].0),
            "marker_kind": kind,
            "marker_path": markers[0].0,
            "members": members,
        });
    }

    let manifest_dirs: Vec<String> = manifests
        .iter()
        .map(|(p, _)| {
            Path::new(p)
                .parent()
                .map(|d| d.display().to_string())
                .unwrap_or_default()
        })
        .collect();

    let distinct_dirs: std::collections::BTreeSet<&String> = manifest_dirs.iter().collect();
    if distinct_dirs.len() >= 2 {
        return json!({
            "is_monorepo": true,
            "reason": "multiple sibling manifest files without workspace declaration",
            "marker_kind": null,
            "marker_path": null,
            "members": manifest_dirs,
        });
    }

    json!({
        "is_monorepo": false,
        "reason": if manifests.is_empty() { "no manifests detected" } else { "single manifest at root or in one directory" },
        "marker_kind": null,
        "marker_path": null,
        "members": [],
    })
}

fn build_applications(
    manifests: &[(String, ParsedManifest)],
    run_configs: &[(String, String, ParsedRunConfig)],
    readmes: &[(String, ReadmeRow)],
) -> Value {
    let mut apps = Vec::new();
    for (m_path, parsed) in manifests {
        let app_dir = Path::new(m_path)
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        let sibling_run_configs: Vec<Value> = run_configs
            .iter()
            .filter(|(p, _, _)| Path::new(p).parent().map(|d| d.display().to_string()) == Some(app_dir.clone()))
            .map(|(p, kind, parsed)| run_config_json(p, kind, parsed))
            .collect();

        let sibling_readme = readmes
            .iter()
            .find(|(p, _)| Path::new(p).parent().map(|d| d.display().to_string()) == Some(app_dir.clone()))
            .map(|(p, r)| {
                json!({
                    "path": p,
                    "line_count": r.line_count,
                    "head": r.text_head,
                })
            });

        let entrypoints = derive_entrypoints(&app_dir, parsed, &sibling_run_configs);
        let ecosystem = match parsed.ecosystem {
            Ecosystem::Rust => "rust",
            Ecosystem::Python => "python",
            Ecosystem::Node => "node",
            Ecosystem::Unknown => "unknown",
        };

        apps.push(json!({
            "root": if app_dir.is_empty() { ".".to_string() } else { app_dir.clone() },
            "manifest": m_path,
            "ecosystem": ecosystem,
            "name": parsed.name,
            "version": parsed.version,
            "language_version": parsed.language_version,
            "scripts": parsed.scripts,
            "top_deps": parsed.top_deps,
            "run_configs": sibling_run_configs,
            "readme": sibling_readme,
            "entrypoint_files": entrypoints,
        }));
    }
    Value::Array(apps)
}

fn derive_entrypoints(
    app_dir: &str,
    parsed: &ParsedManifest,
    run_configs: &[Value],
) -> Value {
    let mut out = Vec::new();
    for s in &parsed.scripts {
        out.push(json!({
            "from": "manifest.scripts",
            "name": s.name,
            "target": s.target,
        }));
    }
    for rc in run_configs {
        if rc.get("kind").and_then(|v| v.as_str()) == Some("Dockerfile") {
            if let Some(cmd) = rc.get("cmd") {
                if !cmd.is_null() {
                    out.push(json!({
                        "from": "Dockerfile.CMD",
                        "value": cmd,
                        "source_path": rc.get("path"),
                    }));
                }
            }
            if let Some(ep) = rc.get("entrypoint") {
                if !ep.is_null() {
                    out.push(json!({
                        "from": "Dockerfile.ENTRYPOINT",
                        "value": ep,
                        "source_path": rc.get("path"),
                    }));
                }
            }
        }
    }
    if out.is_empty() {
        for cand in heuristic_entry_files(app_dir, parsed) {
            out.push(json!({
                "from": "heuristic",
                "path": cand,
            }));
        }
    }
    Value::Array(out)
}

fn heuristic_entry_files(app_dir: &str, parsed: &ParsedManifest) -> Vec<String> {
    let candidates: &[&str] = match parsed.ecosystem {
        Ecosystem::Rust => &["src/main.rs", "src/lib.rs"],
        Ecosystem::Python => &["src/main.py", "main.py", "app.py", "server.py", "__main__.py"],
        Ecosystem::Node => &["src/index.ts", "src/index.js", "index.ts", "index.js", "src/server.ts", "src/server.js"],
        Ecosystem::Unknown => &[],
    };
    candidates
        .iter()
        .map(|c| {
            if app_dir.is_empty() {
                (*c).to_string()
            } else {
                format!("{app_dir}/{c}")
            }
        })
        .collect()
}

fn run_config_json(path: &str, kind: &str, parsed: &ParsedRunConfig) -> Value {
    match parsed {
        ParsedRunConfig::Dockerfile { cmd, entrypoint, expose, from } => json!({
            "path": path,
            "kind": kind,
            "cmd": directive_opt(cmd),
            "entrypoint": directive_opt(entrypoint),
            "expose": expose.iter().map(directive_to_json).collect::<Vec<_>>(),
            "from": directive_opt(from),
        }),
        ParsedRunConfig::Makefile { targets } => json!({
            "path": path,
            "kind": kind,
            "targets": targets.iter().map(directive_to_json).collect::<Vec<_>>(),
        }),
        ParsedRunConfig::Justfile { targets } => json!({
            "path": path,
            "kind": kind,
            "targets": targets.iter().map(directive_to_json).collect::<Vec<_>>(),
        }),
        ParsedRunConfig::Procfile { processes } => json!({
            "path": path,
            "kind": kind,
            "processes": processes.iter().map(directive_to_json).collect::<Vec<_>>(),
        }),
        ParsedRunConfig::DockerCompose { services } => json!({
            "path": path,
            "kind": kind,
            "services": services.iter().map(directive_to_json).collect::<Vec<_>>(),
        }),
    }
}

fn directive_to_json(d: &DirectiveLine) -> Value {
    json!({ "value": d.value, "line": d.line })
}

fn directive_opt(d: &Option<DirectiveLine>) -> Value {
    match d {
        Some(d) => directive_to_json(d),
        None => Value::Null,
    }
}

fn root_level_run_configs(run_configs: &[(String, String, ParsedRunConfig)]) -> Value {
    let items: Vec<Value> = run_configs
        .iter()
        .filter(|(p, _, _)| !p.contains('/'))
        .map(|(p, k, parsed)| run_config_json(p, k, parsed))
        .collect();
    Value::Array(items)
}

pub fn read_span_from_cache(
    cache: &Cache,
    rel: &str,
    start_line: Option<usize>,
    end_line: Option<usize>,
) -> Result<Value> {
    let exists: Option<i64> = cache
        .conn()
        .query_row(
            "SELECT 1 FROM files WHERE path = ?1",
            params![rel],
            |r| r.get(0),
        )
        .ok();
    if exists.is_none() {
        return Ok(json!({ "error": format!("file not tracked: {rel}") }));
    }
    let abs = root_path(cache).join(rel);
    match crate::fs::read_span(&root_path(cache), rel, start_line, end_line) {
        Ok(span) => Ok(json!({
            "path": span.path,
            "start_line": span.start_line,
            "end_line": span.end_line,
            "total_lines": span.total_lines,
            "content": span.content,
        })),
        Err(e) => Ok(json!({ "path": rel, "abs_path": abs.display().to_string(), "error": e })),
    }
}

fn root_path(cache: &Cache) -> PathBuf {
    cache.root.clone()
}

pub struct FileSummaryLookup {
    pub cached: Option<String>,
    pub file_hash: Vec<u8>,
    pub lang: Option<String>,
}

pub fn lookup_file_summary(cache: &Cache, rel: &str) -> Result<Option<FileSummaryLookup>> {
    let row: Option<(Vec<u8>, Option<String>)> = cache
        .conn()
        .query_row(
            "SELECT hash, lang FROM files WHERE path = ?1",
            params![rel],
            |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Option<String>>(1)?)),
        )
        .optional()?;
    let Some((file_hash, lang)) = row else {
        return Ok(None);
    };
    let cached: Option<(String, Vec<u8>)> = cache
        .conn()
        .query_row(
            "SELECT summary, source_hash FROM file_summaries WHERE path = ?1",
            params![rel],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?)),
        )
        .optional()?;
    let cached = cached.and_then(|(s, h)| if h == file_hash { Some(s) } else { None });
    Ok(Some(FileSummaryLookup {
        cached,
        file_hash,
        lang,
    }))
}

pub fn store_file_summary(cache: &Cache, rel: &str, summary: &str, file_hash: &[u8]) -> Result<()> {
    let now = super::now_secs();
    let conn = cache.conn();
    conn.execute(
        "INSERT INTO file_summaries (path, summary, source_hash, generated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(path) DO UPDATE SET summary = excluded.summary, source_hash = excluded.source_hash, generated_at = excluded.generated_at",
        params![rel, summary, file_hash, now],
    )?;
    conn.execute(
        "DELETE FROM file_summaries_fts WHERE path = ?1",
        params![rel],
    )?;
    conn.execute(
        "INSERT INTO file_summaries_fts (path, summary) VALUES (?1, ?2)",
        params![rel, summary],
    )?;
    Ok(())
}

pub struct DirSummaryLookup {
    pub cached: Option<String>,
    pub child_hash: Vec<u8>,
    pub child_file_summaries: Vec<(String, String)>,
}

pub fn lookup_dir_summary(cache: &Cache, dir: &str) -> Result<Option<DirSummaryLookup>> {
    let row: Option<Vec<u8>> = cache
        .conn()
        .query_row(
            "SELECT child_hash FROM dir_hashes WHERE dir_path = ?1",
            params![dir],
            |r| r.get::<_, Vec<u8>>(0),
        )
        .optional()?;
    let Some(child_hash) = row else {
        return Ok(None);
    };
    let cached: Option<(String, Vec<u8>)> = cache
        .conn()
        .query_row(
            "SELECT summary, child_hash FROM dir_summaries WHERE dir_path = ?1",
            params![dir],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?)),
        )
        .optional()?;
    let cached = cached.and_then(|(s, h)| if h == child_hash { Some(s) } else { None });

    let prefix = if dir.is_empty() {
        String::new()
    } else {
        format!("{dir}/")
    };
    let like_pat = format!("{prefix}%");
    let mut stmt = cache.conn().prepare(
        "SELECT fs.path, fs.summary FROM file_summaries fs WHERE fs.path LIKE ?1 ORDER BY fs.path",
    )?;
    let rows = stmt.query_map(params![like_pat], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })?;
    let mut child_file_summaries = Vec::new();
    for row in rows {
        let (p, s) = row?;
        if dir.is_empty() {
            if !p.contains('/') {
                child_file_summaries.push((p, s));
            }
        } else if !p[prefix.len()..].contains('/') {
            child_file_summaries.push((p, s));
        }
    }
    Ok(Some(DirSummaryLookup {
        cached,
        child_hash,
        child_file_summaries,
    }))
}

pub fn store_dir_summary(cache: &Cache, dir: &str, summary: &str, child_hash: &[u8]) -> Result<()> {
    let now = super::now_secs();
    cache.conn().execute(
        "INSERT INTO dir_summaries (dir_path, summary, child_hash, generated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(dir_path) DO UPDATE SET summary = excluded.summary, child_hash = excluded.child_hash, generated_at = excluded.generated_at",
        params![dir, summary, child_hash, now],
    )?;
    Ok(())
}

pub fn semantic_search(cache: &Cache, query: &str, limit: usize) -> Result<Value> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(json!({ "error": "query is empty" }));
    }
    let safe = sanitize_fts_query(q);
    let mut stmt = cache.conn().prepare(
        "SELECT path, summary, bm25(file_summaries_fts) AS rank
         FROM file_summaries_fts
         WHERE file_summaries_fts MATCH ?1
         ORDER BY rank
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![safe, limit as i64], |r| {
        Ok(json!({
            "path": r.get::<_, String>(0)?,
            "summary": r.get::<_, String>(1)?,
            "rank": r.get::<_, f64>(2)?,
        }))
    })?;
    let mut hits = Vec::new();
    for row in rows {
        match row {
            Ok(v) => hits.push(v),
            Err(e) => return Ok(json!({ "error": format!("fts query failed: {e}") })),
        }
    }
    let truncated = hits.len() >= limit;
    Ok(json!({ "hits": hits, "count": hits.len(), "truncated": truncated }))
}

fn sanitize_fts_query(q: &str) -> String {
    // FTS5 treats `:`, `-`, `^`, `(`, `)`, `*`, AND/OR/NOT as syntax. If user query contains
    // any of those (and isn't already quoted), wrap each whitespace-separated token in
    // double quotes to force a phrase match. Keep `*` suffix for prefix queries.
    if q.starts_with('"') {
        return q.to_string();
    }
    let needs_escape = q
        .chars()
        .any(|c| matches!(c, ':' | '-' | '^' | '(' | ')'));
    if !needs_escape {
        return q.to_string();
    }
    q.split_whitespace()
        .map(|tok| {
            let trail_star = tok.ends_with('*');
            let core = if trail_star {
                &tok[..tok.len() - 1]
            } else {
                tok
            };
            let escaped = core.replace('"', "\"\"");
            if trail_star {
                format!("\"{escaped}\"*")
            } else {
                format!("\"{escaped}\"")
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn find_symbol(
    cache: &Cache,
    name: &str,
    kind: Option<&str>,
    path_contains: Option<&str>,
    limit: usize,
) -> Result<Value> {
    let name = name.trim();
    if name.is_empty() {
        return Ok(json!({ "error": "name is empty" }));
    }

    let mut sql = String::from(
        "SELECT path, name, kind, start_line, end_line FROM symbols WHERE name LIKE ?1",
    );
    let mut binds: Vec<String> = vec![format!("%{}%", name)];
    if let Some(k) = kind {
        sql.push_str(" AND kind = ?2");
        binds.push(k.to_string());
        if let Some(p) = path_contains {
            sql.push_str(" AND path LIKE ?3");
            binds.push(format!("%{}%", p));
        }
    } else if let Some(p) = path_contains {
        sql.push_str(" AND path LIKE ?2");
        binds.push(format!("%{}%", p));
    }
    sql.push_str(" ORDER BY name LIMIT ");
    sql.push_str(&limit.to_string());

    let conn = cache.conn();
    let mut stmt = conn.prepare(&sql)?;
    let params_refs: Vec<&dyn rusqlite::ToSql> =
        binds.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

    let rows = stmt.query_map(params_refs.as_slice(), |r| {
        Ok(json!({
            "path": r.get::<_, String>(0)?,
            "name": r.get::<_, String>(1)?,
            "kind": r.get::<_, String>(2)?,
            "start_line": r.get::<_, i64>(3)?,
            "end_line": r.get::<_, i64>(4)?,
        }))
    })?;

    let mut hits = Vec::new();
    for row in rows {
        hits.push(row?);
    }
    let truncated = hits.len() >= limit;
    Ok(json!({ "hits": hits, "count": hits.len(), "truncated": truncated }))
}

// ── inspect_data ──────────────────────────────────────────────────────────────

pub struct InspectData {
    pub total_files: i64,
    pub binary_file_count: i64,
    pub total_size_bytes: i64,
    pub files_by_lang: Vec<(String, i64)>,
    pub newest_file_mtime: Option<i64>,
    pub stale_file_count: Option<i64>,
    pub total_symbols: i64,
    pub symbols_by_kind: Vec<(String, i64)>,
    pub top_symbol_files: Vec<(String, i64)>,
}

pub fn inspect_data(cache: &Cache) -> Result<InspectData> {
    let conn = cache.conn();

    let mut stmt = conn.prepare(
        "SELECT COALESCE(lang, '(none)'), COUNT(*) FROM files GROUP BY lang ORDER BY COUNT(*) DESC",
    )?;
    let files_by_lang: Vec<(String, i64)> = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
        .filter_map(|r| r.ok())
        .collect();
    let total_files: i64 = files_by_lang.iter().map(|(_, c)| c).sum();

    let binary_file_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM files WHERE is_binary = 1", [], |r| r.get(0))?;

    let total_size_bytes: i64 =
        conn.query_row("SELECT COALESCE(SUM(size), 0) FROM files", [], |r| r.get(0))?;

    let newest_file_mtime: Option<i64> =
        conn.query_row("SELECT MAX(mtime) FROM files", [], |r| r.get(0))?;

    let last_scan_at = cache.last_scan_at()?;
    let stale_file_count = last_scan_at.map(|scan_ts| {
        conn.query_row(
            "SELECT COUNT(*) FROM files WHERE mtime > ?1",
            params![scan_ts],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
    });

    let mut stmt = conn.prepare(
        "SELECT kind, COUNT(*) FROM symbols GROUP BY kind ORDER BY COUNT(*) DESC",
    )?;
    let symbols_by_kind: Vec<(String, i64)> = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
        .filter_map(|r| r.ok())
        .collect();
    let total_symbols: i64 = symbols_by_kind.iter().map(|(_, c)| c).sum();

    let mut stmt = conn.prepare(
        "SELECT path, COUNT(*) as n FROM symbols GROUP BY path ORDER BY n DESC LIMIT 5",
    )?;
    let top_symbol_files: Vec<(String, i64)> = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(InspectData {
        total_files,
        binary_file_count,
        total_size_bytes,
        files_by_lang,
        newest_file_mtime,
        stale_file_count,
        total_symbols,
        symbols_by_kind,
        top_symbol_files,
    })
}
