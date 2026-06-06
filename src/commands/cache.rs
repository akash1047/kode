use std::path::Path;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::cache::{self, Cache};
use crate::cache::query;
use crate::cli;

const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1b[0m";

pub fn run(op: cli::CacheOp) {
    match op {
        cli::CacheOp::Build { path } => build(&path),
        cli::CacheOp::Status { path } => status(&path),
        cli::CacheOp::Clear { path } => clear(&path),
        cli::CacheOp::Inspect { path } => inspect(&path),
    }
}

fn build(root: &Path) {
    super::ensure_dir(root);
    let mut c = match Cache::open(root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("cache open failed: {e:#}");
            process::exit(1);
        }
    };
    let report = match cache::revalidate::refresh(&mut c) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("cache refresh failed: {e:#}");
            process::exit(1);
        }
    };
    println!("db: {}", c.db_path.display());
    println!("repo_id: {}", c.repo_id);
    println!(
        "scanned={} new={} changed={} unchanged={} removed={}",
        report.scanned, report.added, report.changed, report.unchanged, report.deleted
    );
    println!(
        "manifests={} run_configs={} readmes={} symbols={} symbol_files={}",
        report.manifests,
        report.run_configs,
        report.readmes,
        report.symbols_total,
        report.symbols_files
    );
}

fn status(root: &Path) {
    super::ensure_dir(root);
    let c = match Cache::open(root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("cache open failed: {e:#}");
            process::exit(1);
        }
    };
    let stats = match c.stats() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("stats failed: {e:#}");
            process::exit(1);
        }
    };
    println!("db: {}", c.db_path.display());
    println!("repo_id: {}", c.repo_id);
    println!("files: {}", stats.files);
    println!("manifests: {}", stats.manifests);
    println!("run_configs: {}", stats.run_configs);
    println!("readmes: {}", stats.readmes);
    println!(
        "symbols: {} ({} files indexed)",
        stats.symbols, stats.symbol_files
    );
    match stats.last_scan_at {
        Some(t) => println!("last_scan_at: {t} (unix)"),
        None => println!("last_scan_at: never"),
    }
}

fn clear(root: &Path) {
    super::ensure_dir(root);
    match cache::clear(root) {
        Ok(p) => {
            if p.exists() {
                eprintln!("clear failed: directory still exists at {}", p.display());
                process::exit(1);
            }
            println!("removed: {}", p.display());
        }
        Err(e) => {
            eprintln!("clear failed: {e:#}");
            process::exit(1);
        }
    }
}

fn inspect(root: &Path) {
    super::ensure_dir(root);
    let c = match Cache::open(root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("cache open failed: {e:#}");
            process::exit(1);
        }
    };
    let data = match query::inspect_data(&c) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("inspect failed: {e:#}");
            process::exit(1);
        }
    };
    let last_scan = c.last_scan_at().ok().flatten();

    println!();
    println!(
        "{BOLD}{CYAN}Cache Inspect{RESET}  {DIM}·{RESET}  {}",
        c.root.display()
    );
    println!("{DIM}db: {}{RESET}", c.db_path.display());
    match last_scan {
        Some(t) => println!("{DIM}last scan:    {}{RESET}", format_age(t)),
        None => println!("{DIM}last scan:    never — run `kode cache build`{RESET}"),
    }
    if let Some(newest) = data.newest_file_mtime {
        println!("{DIM}newest file:  {}{RESET}", format_age(newest));
    }
    if let Some(stale) = data.stale_file_count {
        if stale > 0 {
            println!("{YELLOW}  ⚠  {stale} file(s) modified since last scan — run `kode cache build` to refresh{RESET}");
        } else {
            println!("{DIM}  cache up to date{RESET}");
        }
    }

    // ── Files ──────────────────────────────────────────────────────────────────
    let size_str = format_size(data.total_size_bytes);
    let binary_note = if data.binary_file_count > 0 {
        format!("  {DIM}({} binary){RESET}", data.binary_file_count)
    } else {
        String::new()
    };
    section("Files", data.total_files);
    println!("  {DIM}total size: {size_str}{RESET}{binary_note}");
    let label_w = data.files_by_lang.iter().map(|(l, _)| l.len()).max().unwrap_or(4);
    for (lang, count) in &data.files_by_lang {
        println!("  {CYAN}{lang:<label_w$}{RESET}  {count}");
    }

    // ── Symbols ────────────────────────────────────────────────────────────────
    section("Symbols", data.total_symbols);
    if data.total_symbols == 0 {
        println!("  {DIM}none — run `kode cache build` to index symbols{RESET}");
    } else {
        let label_w = data.symbols_by_kind.iter().map(|(k, _)| k.len()).max().unwrap_or(4);
        for (kind, count) in &data.symbols_by_kind {
            println!("  {YELLOW}{kind:<label_w$}{RESET}  {count}");
        }
        if !data.top_symbol_files.is_empty() {
            println!();
            println!("  {DIM}most symbols:{RESET}");
            for (path, count) in &data.top_symbol_files {
                println!("  {DIM}  {count:>4}  {path}{RESET}");
            }
        }
    }
    println!();
}

fn section(title: &str, count: i64) {
    println!();
    println!("{BOLD}── {title} ({count}) ──{RESET}");
}

fn format_size(bytes: i64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn format_age(unix: i64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let diff = now.saturating_sub(unix);
    if diff < 60 {
        "just now".to_string()
    } else if diff < 3600 {
        format!("{} min ago", diff / 60)
    } else if diff < 86400 {
        format!("{} hr ago", diff / 3600)
    } else {
        format!("{} days ago", diff / 86400)
    }
}
