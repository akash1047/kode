# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-06-01

### Added

- `kode` — project header (name, description, language breakdown) from current directory
- `kode info` — print project header only
- `kode scan` — project header + per-file character counts (gitignore-aware)
- `kode chat` — interactive REPL backed by cached project index and LLM tools
- `kode chat -m` — one-shot question, prints answer and exits
- `kode cache build` — populate cache without entering chat
- `kode cache status` — show DB path, file/symbol/manifest counts, last scan time
- `kode cache inspect` — human-friendly breakdown of cache contents
- `kode cache clear` — delete cache for a repo
- `kode mcp serve` — MCP server exposing `ask` tool (stdio and HTTP transports)
- `kode config init / show / get / set` — manage `~/.config/kode/config.toml`
- SQLite cache (WAL mode) storing files, symbols (Tree-sitter), manifests, run configs, READMEs
- Symbol extraction for Rust, Python, TypeScript, JavaScript, Go, Java, C, C++, C#, Ruby
- Manifest parsing for `Cargo.toml`, `package.json`, `pyproject.toml`
- Run-config parsing for `Dockerfile`, `Makefile`, `Justfile`, `Procfile`, `docker-compose.yml`
- mtime-based cache invalidation — only changed files are re-parsed on subsequent runs

[Unreleased]: https://github.com/akashlohar/kode/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/akashlohar/kode/releases/tag/v0.1.0
