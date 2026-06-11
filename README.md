# kode

[![CI](https://github.com/akash1047/kode/actions/workflows/ci.yml/badge.svg)](https://github.com/akash1047/kode/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/kode.svg)](https://crates.io/crates/kode)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![MSRV: 1.85](https://img.shields.io/badge/rustc-1.85%2B-orange.svg)](rust-toolchain.toml)

Codebase evidence service for humans and AI agents — a local snapshot of any repo, plus a chat REPL where an LLM answers questions about your project using tools that read it for real (no training-data guessing).

## Why kode?

Most LLM tools answer code questions from training data or semantic embeddings, both of which go stale and hallucinate. kode takes a different approach: it caches a parsed, indexed view of your repo on disk and forces every answer to be backed by a live file read with a `path:line` citation. If the source doesn't say it, kode doesn't say it.

## Features

- **Grounded answers** — every claim cites `path:line`; no answer without a file read
- **MCP server** — expose your codebase to any MCP-compatible AI agent over stdio or HTTP
- **Interactive REPL** — `kode chat` opens a terminal chat session with full tool access
- **One-shot queries** — `kode chat -m "..."` prints a single answer and exits
- **Symbol index** — tree-sitter extraction for Rust, Python, TypeScript, JavaScript, Go, Java, C, C++, C#, Ruby
- **Incremental cache** — only changed files are re-parsed on each run (SQLite + xxHash)
- **Manifest awareness** — parses `Cargo.toml`, `package.json`, `pyproject.toml`, Dockerfiles, and more
- **Gitignore-aware** — respects `.gitignore` during all walks

## Installation

build from source:

```sh
git clone https://github.com/akash1047/kode
cd kode
cargo build --release
cargo install --path .
```

## Quick start

```sh
kode                            # project header + file scan in cwd
kode chat                       # interactive chat REPL on cwd
kode chat -m "where is auth?"   # one-shot answer, no REPL
kode mcp serve .                # MCP server for agent callers (stdio)
```

Set your API key:

```sh
kode config init
kode config set api_key YOUR_KEY
```

Full CLI reference: [docs/CLI.md](docs/CLI.md)

## Platform support

| Platform | Status |
|----------|--------|
| Linux | Supported |
| macOS | Supported |
| Windows | Untested |

---

## The cache

Built on first run at `~/.cache/kode/<repo-id>/index.db` (SQLite, WAL mode). Only changed files are re-parsed on subsequent runs.

| Layer | Contents |
|-------|----------|
| Files | Paths, hashes, mtimes, sizes, langs |
| Symbols | Tree-sitter-extracted functions/classes/types (Rust, Python, TypeScript, JavaScript, Go, Java, C, C++, C#, Ruby) |
| Manifests | `Cargo.toml`, `package.json`, `pyproject.toml` |
| Run configs | `Dockerfile`, `Makefile`, `Justfile`, `Procfile`, `docker-compose.yml` |
| READMEs | First 80 lines |

Cache key: hash of `git remote origin` if present, else hash of canonical abs path.

```sh
kode cache status               # inspect stats
kode cache clear                # delete cache for this repo
```

---

## Documentation

- [docs/CLI.md](docs/CLI.md) — full CLI reference, config file, environment variables
- [DESIGN.md](DESIGN.md) — architecture, contracts, cache discipline, design principles

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| [`ignore`](https://crates.io/crates/ignore) | gitignore-aware walker |
| [`adk-rust`](https://crates.io/crates/adk-rust) | agent / tool-calling framework |
| [`tokio`](https://crates.io/crates/tokio) | async runtime |
| [`reedline`](https://crates.io/crates/reedline) | REPL line editor |
| [`termimad`](https://crates.io/crates/termimad) | markdown rendering in terminal |
| [`rusqlite`](https://crates.io/crates/rusqlite) | cache storage (bundled sqlite) |
| [`xxhash-rust`](https://crates.io/crates/xxhash-rust) | fast non-cryptographic file hashing |
| [`toml`](https://crates.io/crates/toml) / [`serde_yaml`](https://crates.io/crates/serde_yaml) / [`serde_json`](https://crates.io/crates/serde_json) | manifest + run-config parsing |
| [`dirs`](https://crates.io/crates/dirs) | XDG cache/config dir resolution |
| [`regex`](https://crates.io/crates/regex) | `search_project` tool |
| [`tree-sitter`](https://crates.io/crates/tree-sitter) + grammars | symbol extraction (Rust, Python, TypeScript, JavaScript, Go, Java, C, C++, C#, Ruby) |
| [`reqwest`](https://crates.io/crates/reqwest) | LLM HTTP client |
| [`clap`](https://crates.io/crates/clap) | CLI parsing |
| [`axum`](https://crates.io/crates/axum) | MCP HTTP transport |

## Contributing

Contributions welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## License

MIT
