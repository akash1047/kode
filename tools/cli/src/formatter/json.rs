use crate::presenter::cache::CacheStatusView;
use crate::presenter::files::FilesView;
use crate::presenter::scan::ScanView;
use crate::presenter::status::StatusView;
use crate::presenter::symbols::SymbolsView;

pub fn format_scan(view: &ScanView) -> String {
    let json = serde_json::json!({
        "repository_root": view.repository_root,
        "workspace": view.workspace,
        "files_discovered": view.files_discovered,
        "directories": view.directories,
        "languages": view.languages,
        "manifests": view.manifests,
        "parsed": view.parsed,
        "recovered": view.recovered,
        "skipped": view.skipped,
        "failed": view.failed,
        "elapsed_secs": view.elapsed_secs,
        "cache_hit": view.cache_hit,
    });
    serde_json::to_string_pretty(&json).unwrap_or_else(|_| "{}".into())
}

pub fn format_status(view: &StatusView) -> String {
    let json = serde_json::json!({
        "repository_root": view.repository_root,
        "workspace": view.workspace,
        "files": view.files,
        "languages": view.languages,
        "parsed": view.parsed,
        "recovered": view.recovered,
        "skipped": view.skipped,
        "failed": view.failed,
        "source": view.source,
        "graph_nodes": view.graph_nodes,
        "graph_relationships": view.graph_relationships,
        "revision": view.revision,
    });
    serde_json::to_string_pretty(&json).unwrap_or_else(|_| "{}".into())
}

pub fn format_files(view: &FilesView) -> String {
    let entries: Vec<serde_json::Value> = view
        .entries
        .iter()
        .map(|(lang, path)| {
            serde_json::json!({
                "language": lang,
                "path": path,
            })
        })
        .collect();
    let json = serde_json::json!({
        "files": entries,
    });
    serde_json::to_string_pretty(&json).unwrap_or_else(|_| "{}".into())
}

pub fn format_symbols(view: &SymbolsView) -> String {
    let entries: Vec<serde_json::Value> = view
        .entries
        .iter()
        .map(|(name, kind, file, line, verified)| {
            serde_json::json!({
                "name": name,
                "kind": kind,
                "file": file,
                "line": line,
                "verified": verified,
            })
        })
        .collect();
    let json = serde_json::json!({
        "symbols": entries,
    });
    serde_json::to_string_pretty(&json).unwrap_or_else(|_| "{}".into())
}

pub fn format_cache(view: &CacheStatusView) -> String {
    let json = serde_json::json!({
        "repository_id": view.repository_id,
        "revision_count": view.revision_count,
        "total_nodes": view.total_nodes,
        "total_relationships": view.total_relationships,
        "schema_version": view.schema_version,
        "graph_version": view.graph_version,
        "latest_revision": view.latest_revision,
    });
    serde_json::to_string_pretty(&json).unwrap_or_else(|_| "{}".into())
}
