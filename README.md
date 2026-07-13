# kode

[![CI](https://github.com/akash1047/kode/actions/workflows/ci.yml/badge.svg)](https://github.com/akash1047/kode/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![MSRV: 1.85](https://img.shields.io/badge/rustc-1.85%2B-orange.svg)](rust-toolchain.toml)

**Evidence-first code intelligence for humans and AI agents.**

kode builds a deterministic understanding of your repository and answers questions using **live source code** with **`path:line` citations**. Every answer is grounded in the repository—not training data, embeddings, or guesswork.

> **If the source doesn't say it, kode doesn't say it.**

---

## Current Status

**Product-complete for Rust repositories** through scan → graph → store → query → CLI / MCP / chat.

| Subsystem | Status |
|-----------|--------|
| Acquisition (discovery, gitignore, kodeignore) | Implemented |
| Parsing + fact extraction (Rust / tree-sitter) | Implemented (incl. call sites) |
| Knowledge graph + validation | Implemented (`Contains`, `Declares`, `Defines`, `Calls`) |
| Storage (SQLite revisions) | Implemented |
| Query engine | Implemented (search, call graph, impact, metrics, dead, cycles) |
| Incremental skip | Whole-repo fingerprint (skip rebuild when unchanged) |
| CLI | Full command surface |
| MCP | 8 tools |
| Chat agent TUI | FS tools + symbol/call/metrics tools |

Still open (see [docs/ROADMAP.md](docs/ROADMAP.md)):

- Per-file incremental reparse
- Multi-language parsers beyond Rust
- `--watch` and parallel `--threads`
- Import dependency edges, LSP / IDE / web

Contributors should read `DESIGN.md` and the [documentation index](docs/README.md).

---

## Why kode?

Most AI coding assistants rely on training data and/or embeddings. Neither guarantees answers reflect the **current** repository.

kode parses your project, builds a deterministic **Knowledge Graph**, persists it locally, and requires evidence from live source files.

The LLM never invents repository facts. It uses tools and the graph to locate code, then reads the files.

---

## Features

**Implemented:**

* Gitignore / `.kodeignore`-aware filesystem traversal
* Cargo workspace and manifest discovery
* Language detection by extension (parse/extract: **Rust**)
* Deterministic knowledge graph with evidence-backed nodes
* Call graph + impact analysis + fan-in/out metrics + cycle detection
* SQLite revision cache with fingerprint-based incremental skip
* CLI: `scan`, `status`, `files`, `symbols`, `query`, `export`, `chat`, `cache`, `config`, `mcp`
* MCP server for external agents
* Agent chat TUI with sandboxed tools

**Planned / incomplete:**

* Per-file incremental indexing
* Additional language extractors (Python, TypeScript, …)
* `--watch` mode and multi-threaded scan
* LSP / IDE plugins / web UI

---

## Installation

```sh
git clone https://github.com/akash1047/kode
cd kode
cargo build --release
cargo install --path tools/cli
```

---

## Quick Start

```sh
kode config init
kode scan
kode status
kode query "run_scan"
kode query "callers:run_scan"
kode query "metrics"
kode chat -m "Where is the scan pipeline orchestrated?"
```

Cache lives at `<repo>/.kode/cache.db`. Second `kode scan` with no file changes reports a **cache hit**.

See [docs/CLI.md](docs/CLI.md) for the full command reference.

---

## How It Works

```text
Repository
     │
     ▼
RepositoryDiscovery     ◄── Acquisition
     │
     ▼
Parsing + Fact Extraction  ◄── Analysis (Rust)
     │
     ▼
Knowledge Graph         ◄── Graph
     │
     ▼
Graph Revision          ◄── Storage (.kode/cache.db)
     │
     ├── Query Engine   ◄── search / callers / impact / metrics
     └── Interfaces     ◄── CLI · MCP · Chat agent
```

---

## Query cheatsheet

```sh
kode query "SymbolName"       # prefix / exact search
kode query "callers:foo"
kode query "callees:foo"
kode query "impact:foo"       # reverse call BFS
kode query "metrics"
kode query "dead"             # no incoming calls (heuristic)
kode query "cycles"
```

---

## Project Philosophy

* **Evidence over confidence** — every claim should cite source
* **Deterministic by default** — parsers and algorithms, not LLM invention
* **Local-first** — indexes stay on your machine
* **Incremental** — skip work when the repo is unchanged
* **Composable** — same model powers CLI, MCP, and chat

---

## Documentation

See the [documentation index](docs/README.md).

| Document | Purpose |
|----------|---------|
| `DESIGN.md` | System design |
| `docs/ROADMAP.md` | Milestones |
| `docs/CLI.md` | Command reference |
| `docs/ARCHITECTURE.md` | Architecture |
| `docs/CHAT_UI.md` | Chat agent |

---

## Contributing

Read `DESIGN.md`, [GLOSSARY](docs/GLOSSARY.md), [GUIDELINES](docs/GUIDELINES.md), and [CRATE_OVERVIEW](docs/CRATE_OVERVIEW.md).

Preserve: deterministic understanding, evidence-backed answers, reproducible analysis.

---

## License

MIT
