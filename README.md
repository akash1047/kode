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

The **Acquisition**, **Parsing**, and **Fact Extraction** subsystems are implemented.

Acquisition discovers repository structure and produces an immutable structural snapshot:

- Repository discovery orchestration
- Workspace detection
- Filesystem traversal (.gitignore-aware)
- Manifest discovery
- Language detection
- RepositorySnapshot with typed inventories
- Detector architecture with registries (WorkspaceDetector, ManifestDetector, LanguageDetector)
- Default detectors for Cargo workspaces, Cargo manifests, and extension-based language detection

Parsing transforms source files into language-specific syntax trees:

- `Parser` trait for language-specific parsers
- `ParserRegistry` with language-keyed dispatch
- `RustParser` — Tree-sitter-backed Rust parser
- `ParsingOrchestrator` — pure transformation over immutable inputs
- `SyntaxTree` and `SyntaxTreeInventory` — immutable domain artifacts

Fact Extraction transforms syntax trees into language-independent repository facts:

- `Extractor` trait for language-specific extractors
- `ExtractorRegistry` with language-keyed dispatch
- `RustExtractor` — Tree-sitter-backed Rust extractor
- `ExtractionOrchestrator` — pure transformation over immutable inputs
- `RepositoryFacts` — immutable domain artifact with entities and evidence
- `EntityId` — stable, deterministic hash-based entity identifiers
- `Evidence` — source location evidence attached to every entity

The CLI command interface (argument parsing, help generation, command dispatch) is also implemented. The `scan` subcommand has not yet been wired to the acquisition library — all subcommands currently return placeholder messages.

The following capabilities are **planned** and not yet implemented:

- Knowledge Graph construction
- Incremental caching and storage
- Query execution
- Analysis engine (graph algorithms)
- MCP server logic
- LLM integration

Contributors should read `DESIGN.md`, [ACQUISITION.md](docs/ACQUISITION.md), [ANALYSIS.md](docs/ANALYSIS.md), and the [documentation index](docs/README.md) before starting work.

---

## Why kode?

Most AI coding assistants rely on one or both of these approaches:

* Training data that may be outdated.
* Semantic embeddings that trade precision for similarity.

Neither guarantees that an answer reflects the current state of your repository.

kode takes a different approach.

It parses your project, builds a deterministic **Knowledge Graph** from source code and project metadata, persists it locally for incremental updates, and requires every answer to be verified against the repository before it is returned.
The Acquisition, Parsing, and Fact Extraction subsystems are implemented. The Knowledge Graph and downstream stages are planned.

The Knowledge Graph is an implementation detail—not an AI-generated artifact. It is produced entirely through deterministic parsing and analysis.

The LLM never invents facts. It uses the graph to locate relevant parts of the repository, reads the source files, and produces evidence-backed answers.

---

## Features

**Implemented:**

* **Gitignore-aware filesystem traversal** — respects project boundaries automatically
* **Workspace detection** — identifies single-package and multi-package Cargo workspaces
* **Manifest discovery** — detects build configuration and package manifests
* **Language detection** — classifies files by programming language via extension mapping
* **RepositorySnapshot** — immutable structural snapshot of the repository with typed inventories
* **Detector architecture** — composable, registry-based extension model
* **Parsing** — language-specific syntax tree generation via tree-sitter
* **Fact Extraction** — language-independent repository facts with evidence
* **RepositoryFacts** — immutable domain artifact with EntityId and Evidence

**Planned:**

* **Evidence-backed answers** — every claim includes `path:line` citations
* **Deterministic Knowledge Graph** — built from repository facts
* **Incremental indexing** — only changed files are reparsed
* **Interactive REPL** — terminal chat with full repository awareness
* **One-shot queries** — ask a question and exit
* **MCP server** — expose your repository to any MCP-compatible AI agent
* **Additional language support** — Python, TypeScript, JavaScript, Go, Java and more
* **Manifest awareness** — npm, Python, Docker and more
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
```

Scan the current repository (placeholder — not yet wired to the acquisition library):

```sh
kode scan
```

All subcommands currently validate arguments and return a placeholder message.
Full repository processing is under development. See [docs/ACQUISITION.md](docs/ACQUISITION.md) for the implemented Acquisition subsystem.

---

## How It Works

```text
Repository
     │
     ▼
RepositoryDiscovery
     │
     ▼
RepositorySnapshot          ◄── Acquisition boundary (implemented)
     │
     ▼
Parsing                     ─── Implemented (Analysis)
     │
     ▼
Syntax Trees
     │
     ▼
Fact Extraction             ─── Implemented (Analysis)
     │
     ▼
RepositoryFacts
     │
     ▼
Knowledge Graph             ─── Planned
     │
     ├── Storage
     ├── Query Engine
     ├── Analysis (graph algorithms)
     └── Interfaces
```

The Acquisition boundary produces an **immutable structural snapshot of the repository** containing workspace structure, files, directories, manifests, and language inventories.

Parsing and Fact Extraction are implemented in the Analysis subsystem — see [ANALYSIS.md](docs/ANALYSIS.md) and [PIPELINE.md](docs/PIPELINE.md).

When the full pipeline is implemented, kode will use the graph to efficiently locate relevant code, then read original source files to verify every claim before returning an answer.

---

## Evidence First (Planned)

When the full pipeline is implemented, every answer will follow the same process:

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

## Incremental Cache (Planned)

kode will store repository metadata locally to avoid reparsing unchanged files.

Default location:

```text
~/.cache/kode/<repository-id>/
```

The cache is planned to store:

* File metadata
* File hashes
* Modification timestamps
* Extracted symbols
* Project manifests
* Repository metadata

Incremental caching is not yet implemented.

---

## Commands

All commands currently accept arguments and return a placeholder message.

```sh
kode scan
```

Discover repository structure and produce a RepositorySnapshot (intended behavior — not yet wired).

```sh
kode status
```

Show repository indexing status (planned).

```sh
kode files
```

Explore repository files (planned).

```sh
kode symbols
```

Explore extracted symbols (planned).

```sh
kode query "<query>"
```

Query repository knowledge (planned).

```sh
kode chat
```

Interactive chat session (planned).

```sh
kode chat -m "..."
```

Ask a single question (planned).

```sh
kode mcp serve .
```

Run the MCP server (planned).

```sh
kode cache status
```

Inspect repository cache (planned).

```sh
kode cache clear
```

Remove the cache for the current repository (planned).

```sh
kode config init
```

Initialize configuration.

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

Only modified files will be reparsed.

### Composable

The same repository model powers chat, MCP, CLI tools, analysis, and future integrations.

---

## Documentation

See the [documentation index](docs/README.md) for a complete map of all documents, recommended reading order, and one-line descriptions.

| Document | Purpose |
|----------|---------|
| `DESIGN.md` | System design and architecture |
| `docs/ARCHITECTURE.md` | High-level architecture |
| `docs/ACQUISITION.md` | Acquisition subsystem — current implementation |
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

Please read `DESIGN.md`, the [Glossary](docs/GLOSSARY.md), the [CRATE_OVERVIEW.md](docs/CRATE_OVERVIEW.md), the [documentation index](docs/README.md), and the [workspace guidelines](docs/GUIDELINES.md) before contributing.

Local CI validation with `act` is documented in the
[workspace guidelines](docs/GUIDELINES.md).

Every contribution should preserve kode's core guarantees:

* Deterministic repository understanding
* Evidence-backed answers
* Incremental processing
* Reproducible analysis

---

## License

MIT

