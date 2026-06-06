pub const SCHEMA_VERSION: i32 = 3;

pub const DDL: &str = r#"
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS repo (
    id              TEXT PRIMARY KEY,
    abs_path        TEXT NOT NULL,
    git_remote      TEXT,
    last_scan_at    INTEGER
);

CREATE TABLE IF NOT EXISTS files (
    path            TEXT PRIMARY KEY,
    hash            BLOB NOT NULL,
    mtime           INTEGER NOT NULL,
    size            INTEGER NOT NULL,
    lang            TEXT,
    is_binary       INTEGER NOT NULL DEFAULT 0,
    updated_at      INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS files_lang ON files(lang);

CREATE TABLE IF NOT EXISTS manifests (
    path            TEXT PRIMARY KEY REFERENCES files(path) ON DELETE CASCADE,
    parsed_json     TEXT NOT NULL,
    hash            BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS run_configs (
    path            TEXT PRIMARY KEY REFERENCES files(path) ON DELETE CASCADE,
    kind            TEXT NOT NULL,
    parsed_json     TEXT NOT NULL,
    hash            BLOB NOT NULL
);

CREATE INDEX IF NOT EXISTS run_configs_kind ON run_configs(kind);

CREATE TABLE IF NOT EXISTS readmes (
    path            TEXT PRIMARY KEY REFERENCES files(path) ON DELETE CASCADE,
    text_head       TEXT NOT NULL,
    line_count      INTEGER NOT NULL,
    hash            BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS symbols (
    path            TEXT NOT NULL REFERENCES files(path) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    kind            TEXT NOT NULL,
    start_line      INTEGER NOT NULL,
    end_line        INTEGER NOT NULL,
    signature       TEXT
);

CREATE INDEX IF NOT EXISTS symbols_path ON symbols(path);
CREATE INDEX IF NOT EXISTS symbols_name ON symbols(name);
CREATE INDEX IF NOT EXISTS symbols_kind ON symbols(kind);

CREATE TABLE IF NOT EXISTS dir_hashes (
    dir_path        TEXT PRIMARY KEY,
    child_hash      BLOB NOT NULL,
    file_count      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS file_summaries (
    path            TEXT PRIMARY KEY REFERENCES files(path) ON DELETE CASCADE,
    summary         TEXT NOT NULL,
    source_hash     BLOB NOT NULL,
    generated_at    INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS dir_summaries (
    dir_path        TEXT PRIMARY KEY,
    summary         TEXT NOT NULL,
    child_hash      BLOB NOT NULL,
    generated_at    INTEGER NOT NULL
);

CREATE VIRTUAL TABLE IF NOT EXISTS file_summaries_fts USING fts5(
    path UNINDEXED,
    summary,
    tokenize = 'porter unicode61'
);
"#;
