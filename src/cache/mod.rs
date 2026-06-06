pub mod hash;
pub mod query;
pub mod revalidate;
mod schema;
pub mod summarize;
pub mod symbols;

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};

use crate::project;

/// SQLite-backed cache for a single repository.
///
/// One `Cache` instance maps to one `index.db` under `~/.cache/kode/<repo-id>/`.
/// Call [`Cache::open`] to create or reopen; the DB schema is applied automatically.
pub struct Cache {
    conn: Connection,
    /// Stable identifier for this repo (derived from git remote or canonical path).
    pub repo_id: String,
    /// Absolute path to the SQLite database file.
    pub db_path: PathBuf,
    /// Canonical absolute path to the project root.
    pub root: PathBuf,
}

impl Cache {
    /// Open (or create) the cache for the repository at `root`.
    ///
    /// Canonicalises the path, derives the repo ID, creates the cache directory,
    /// opens the SQLite connection, applies schema migrations, and upserts the
    /// repo row.
    pub fn open(root: &Path) -> Result<Self> {
        let abs_root = std::fs::canonicalize(root)
            .with_context(|| format!("canonicalize {}", root.display()))?;

        let info = project::info(&abs_root);
        let repo_id = hash::repo_id_from(info.remote_url.as_deref(), &abs_root);

        let db_dir = cache_dir()?.join(&repo_id);
        std::fs::create_dir_all(&db_dir)
            .with_context(|| format!("mkdir {}", db_dir.display()))?;
        let db_path = db_dir.join("index.db");

        let conn = Connection::open(&db_path)
            .with_context(|| format!("open sqlite {}", db_path.display()))?;
        conn.execute_batch(schema::DDL)?;
        migrate(&conn)?;

        conn.execute(
            "INSERT INTO repo (id, abs_path, git_remote, last_scan_at)
             VALUES (?1, ?2, ?3, NULL)
             ON CONFLICT(id) DO UPDATE SET abs_path = excluded.abs_path, git_remote = excluded.git_remote",
            params![
                &repo_id,
                abs_root.display().to_string(),
                info.remote_url.as_deref(),
            ],
        )?;

        Ok(Self {
            conn,
            repo_id,
            db_path,
            root: abs_root,
        })
    }

    /// Shared reference to the underlying SQLite connection.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Exclusive reference to the underlying SQLite connection.
    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Record the current time as the last-scan timestamp for this repo.
    pub fn mark_scanned(&self) -> Result<()> {
        let now = now_secs();
        self.conn.execute(
            "UPDATE repo SET last_scan_at = ?1 WHERE id = ?2",
            params![now, &self.repo_id],
        )?;
        Ok(())
    }

    /// Returns the Unix timestamp (seconds) of the last completed scan, or `None` if never scanned.
    pub fn last_scan_at(&self) -> Result<Option<i64>> {
        let v: Option<i64> = self
            .conn
            .query_row(
                "SELECT last_scan_at FROM repo WHERE id = ?1",
                params![&self.repo_id],
                |r| r.get(0),
            )
            .optional()?
            .flatten();
        Ok(v)
    }

    /// Collect row counts and last-scan time from the cache DB.
    pub fn stats(&self) -> Result<CacheStats> {
        let files: i64 = self.conn.query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))?;
        let manifests: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM manifests", [], |r| r.get(0))?;
        let run_configs: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM run_configs", [], |r| r.get(0))?;
        let readmes: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM readmes", [], |r| r.get(0))?;
        let symbols: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM symbols", [], |r| r.get(0))?;
        let symbol_files: i64 = self
            .conn
            .query_row("SELECT COUNT(DISTINCT path) FROM symbols", [], |r| r.get(0))?;
        let last_scan_at = self.last_scan_at()?;
        Ok(CacheStats {
            files,
            manifests,
            run_configs,
            readmes,
            symbols,
            symbol_files,
            last_scan_at,
        })
    }
}

/// Aggregate statistics read from the cache database.
#[derive(Debug)]
pub struct CacheStats {
    /// Number of tracked source files.
    pub files: i64,
    /// Number of parsed manifest files (`Cargo.toml`, `package.json`, etc.).
    pub manifests: i64,
    /// Number of parsed run-config files (`Dockerfile`, `Makefile`, etc.).
    pub run_configs: i64,
    /// Number of cached README excerpts.
    pub readmes: i64,
    /// Total number of extracted symbols across all files.
    pub symbols: i64,
    /// Number of distinct files that contributed at least one symbol.
    pub symbol_files: i64,
    /// Unix timestamp of the last completed scan, or `None` if never scanned.
    pub last_scan_at: Option<i64>,
}

/// Delete the entire cache directory for the repository at `root`.
///
/// Returns the path that was removed (useful for user-facing output).
pub fn clear(root: &Path) -> Result<PathBuf> {
    let abs_root = std::fs::canonicalize(root)
        .with_context(|| format!("canonicalize {}", root.display()))?;
    let info = project::info(&abs_root);
    let repo_id = hash::repo_id_from(info.remote_url.as_deref(), &abs_root);
    let db_dir = cache_dir()?.join(&repo_id);
    if db_dir.exists() {
        std::fs::remove_dir_all(&db_dir)
            .with_context(|| format!("rm -rf {}", db_dir.display()))?;
    }
    Ok(db_dir)
}

fn cache_dir() -> Result<PathBuf> {
    let base = dirs::cache_dir().context("XDG cache dir not resolvable")?;
    Ok(base.join("kode"))
}

fn migrate(conn: &Connection) -> Result<()> {
    let current: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if current < schema::SCHEMA_VERSION {
        conn.execute_batch(&format!("PRAGMA user_version = {}", schema::SCHEMA_VERSION))?;
    }
    Ok(())
}

/// Current Unix timestamp in whole seconds.
pub fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
