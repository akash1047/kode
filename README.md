# kode

[![CI](https://github.com/akash1047/kode/actions/workflows/ci.yml/badge.svg)](https://github.com/akash1047/kode/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/kode.svg)](https://crates.io/crates/kode)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![MSRV: 1.85](https://img.shields.io/badge/rustc-1.85%2B-orange.svg)](rust-toolchain.toml)

**Evidence-first code intelligence for humans and AI agents.**

kode builds a deterministic understanding of your repository and answers questions using **live source code** with **`path:line` citations**. Every answer is grounded in the repository—not training data, embeddings, or guesswork.

> **If the source doesn't say it, kode doesn't say it.**

---

## Current Status

The CLI foundation and public command interface are **implemented**. The full command hierarchy, global options, help generation, command dispatch, and placeholder handlers are in place. The CLI establishes the public contract for all kode operations.

Repository processing **is not yet implemented**. The following capabilities remain under development:

- Repository discovery and scanning
- Language parsing (Tree-sitter)
- Knowledge Graph construction
- Incremental caching and storage
- Query execution
- Analysis engine
- MCP server logic
- LLM integration

Running a kode command will:
- Parse arguments and validate input
- Generate help output when requested
- Dispatch to the appropriate subcommand handler
- Return a placeholder message confirming the command was received

No repository processing, file parsing, knowledge graph construction, storage, or LLM integration has been implemented yet.

Contributors should read `DESIGN.md` and the documentation index in `docs/README.md` before starting work.

---

## Why kode?

Most AI coding assistants rely on one or both of these approaches:

* Training data that may be outdated.
* Semantic embeddings that trade precision for similarity.

Neither guarantees that an answer reflects the current state of your repository.

kode takes a different approach.

It parses your project, builds a deterministic **Knowledge Graph** from source code and project metadata, persists it locally for incremental updates, and requires every answer to be verified against the repository before it is returned.

The Knowledge Graph is an implementation detail—not an AI-generated artifact. It is produced entirely through deterministic parsing and analysis.

The LLM never invents facts. It uses the graph to locate relevant parts of the repository, reads the source files, and produces evidence-backed answers.

---

## Features

* **Evidence-backed answers** — every claim includes `path:line` citations
* **Deterministic Knowledge Graph** — built from source code and project metadata
* **Incremental indexing** — only changed files are reparsed
* **Interactive REPL** — terminal chat with full repository awareness
* **One-shot queries** — ask a question and exit
* **MCP server** — expose your repository to any MCP-compatible AI agent
* **Tree-sitter symbol extraction** — Rust, Python, TypeScript, JavaScript, Go, Java, C, C++, C#, Ruby
* **Manifest awareness** — Cargo, npm, Python, Docker and more
* **Gitignore-aware** — respects project boundaries automatically
* **Local-first** — indexes remain on your machine

---

## Installation

Build from source:

```sh
git clone https://github.com/akash1047/kode
cd kode

cargo build --release
cargo install --path .
```

---

## Quick Start

Initialize configuration:

```sh
kode config init
kode config set api_key YOUR_KEY
```

Scan the current repository:

```sh
kode scan
```

Start an interactive chat session:

```sh
kode chat
```

Ask a single question:

```sh
kode chat -m "Where is authentication implemented?"
```

Run the MCP server:

```sh
kode mcp serve .
```

Check cache status:

```sh
kode cache status
```

Clear the repository cache:

```sh
kode cache clear
```

---

## How It Works

```text
Repository
     │
     ▼
Repository Discovery
     │
     ▼
Language Parsers
(Tree-sitter, manifests, configs)
     │
     ▼
Knowledge Graph
     │
     ├── Incremental SQLite Cache
     ├── Query Engine
     ├── Analysis
     ├── MCP
     ├── CLI
     └── LLM
```

The Knowledge Graph is the canonical representation of the repository.

It is built entirely through deterministic parsing and analysis.

When answering a question, kode uses the graph to efficiently locate relevant code, then reads the original source files to verify every claim before returning an answer.

---

## Evidence First

Every answer follows the same process:

```text
Question
    │
    ▼
Knowledge Graph
    │
    ▼
Locate Relevant Code
    │
    ▼
Read Source Files
    │
    ▼
Generate Answer
    │
    ▼
Attach path:line Citations
```

The graph narrows the search.

The source code remains the final authority.

---

## Incremental Cache

kode stores repository metadata locally to avoid reparsing unchanged files.

Default location:

```text
~/.cache/kode/<repository-id>/
```

The cache currently stores:

* File metadata
* File hashes
* Modification timestamps
* Extracted symbols
* Project manifests
* Repository metadata

Future releases will extend the cache with richer repository relationships and analysis.

---

## Commands

```sh
kode scan
```

Discover and index a repository.

```sh
kode status
```

Show repository indexing status.

```sh
kode files
```

Explore indexed repository files.

```sh
kode symbols
```

Explore extracted symbols.

```sh
kode query "<query>"
```

Query repository knowledge.

```sh
kode chat
```

Interactive chat session.

```sh
kode chat -m "..."
```

Ask a single question.

```sh
kode mcp serve .
```

Run the MCP server.

```sh
kode cache status
```

Inspect repository cache.

```sh
kode cache clear
```

Remove the cache for the current repository.

```sh
kode config init
```

Initialize configuration.

```sh
kode config set <key> <value>
```

Set a configuration value.

---

## Project Philosophy

kode is built on a few simple principles.

### Evidence over confidence

Every answer must be backed by source code.

### Deterministic by default

Repository understanding is produced by parsers and algorithms—not AI.

### Local-first

Repository indexes stay on your machine.

### Incremental

Only modified files are reparsed.

### Composable

The same repository model powers chat, MCP, CLI tools, analysis, and future integrations.

---

## Documentation

See the [documentation index](docs/README.md) for a complete map of all documents, recommended reading order, and one-line descriptions.

| Document | Purpose |
|----------|---------|
| `DESIGN.md` | System design and architecture |
| `docs/ARCHITECTURE.md` | High-level architecture |
| `docs/PIPELINE.md` | Repository processing pipeline |
| `docs/KNOWLEDGE_GRAPH.md` | Knowledge Graph specification |
| `docs/STORAGE.md` | Persistence and caching |
| `docs/QUERY_ENGINE.md` | Query execution |
| `docs/ANALYSIS.md` | Analysis engine |
| `docs/EVIDENCE_MODEL.md` | Evidence guarantees |
| `docs/MCP.md` | MCP architecture |
| `docs/CLI.md` | CLI reference |
| `docs/GUIDELINES.md` | Workspace conventions |
| `docs/GLOSSARY.md` | Terminology |
| `docs/ROADMAP.md` | Future direction |
| `docs/CRATE_OVERVIEW.md` | Crate responsibilities |

---

## Roadmap

Planned capabilities are documented in [docs/ROADMAP.md](docs/ROADMAP.md).

---

## Contributing

Contributions are welcome.

Please read `DESIGN.md`, the [Glossary](docs/GLOSSARY.md), the [CRATE_OVERVIEW.md](docs/CRATE_OVERVIEW.md), and the documentation index in [docs/README.md](docs/README.md) before contributing.

Every contribution should preserve kode's core guarantees:

* Deterministic repository understanding
* Evidence-backed answers
* Incremental processing
* Reproducible analysis

---

## License

MIT

