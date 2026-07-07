# Crate Overview

This document describes every crate in the **kode** workspace.

Each entry distinguishes between the current state (what exists today) and the planned responsibility (the long-term architectural intent).

---

## Workspace Layout

```
kode/
├── crates/               # Reusable library crates
│   ├── acquisition/      # kode-acquisition
│   ├── graph/            # kode-graph
│   ├── storage/          # kode-storage
│   ├── analysis/         # kode-analysis
│   ├── query/            # kode-query
│   └── common/           # kode-common
├── services/             # Long-running applications
│   └── app/              # kode-app
└── tools/                # CLI utilities
    └── cli/              # kode-cli
```

---

## Dependency Flow

```
Applications (tools/cli, services/app)
    │
    └── Library Crates (crates/)
```

Dependencies flow downward. No circular dependencies are permitted.

`kode-common` is the lowest-level library crate; all other crates may depend on it.

---

## Crate Reference

### kode-acquisition

**Path:** `crates/acquisition/`

**Current State**
- Repository domain model and snapshot construction
- Repository discovery orchestration (workspace detection, filesystem traversal, manifest discovery, language detection)
- Detector architecture with composable, registry-based extension
- Default detectors for Cargo workspaces, manifests, and extension-based language detection
- Validated snapshot construction

**Planned Responsibility**
- (none — downstream processing is handled by kode-analysis)

---

### kode-graph

**Path:** `crates/graph/`

**Current State**
- Stage 4 (Graph Construction) fully implemented
- Immutable, validated, traversable in-memory graph with structural and entity nodes
- Dual identity model separating synthetic node identities from extracted entity identities
- Dual evidence model with source-backed and structural evidence
- Directed relationship model with typed relationship kinds
- Decomposed graph builder performing zero repository discovery
- Topology validation (structural roots, orphan detection, duplicate safety net)
- Graph serialization support (deterministic round-trip conversion)
- Integration tests covering structure, determinism, evidence, traversal, and edge cases

**Planned Responsibility**
- Additional index structures (name, qualified name, file lookup)

---

### kode-storage

**Path:** `crates/storage/`

**Current State**
- Stage 5 (Persistence) fully implemented
- Backend abstraction with methods for initialization, revision persistence, graph loading, revision listing, and repository removal
- Production SQLite backend with schema management, transactional persistence, content hashing, and revision tracking
- Repository-scoped storage service binding a backend to a repository ID
- Schema version tracking, immutable revision model, and cache metadata
- Delegates graph serialization to the graph crate's serialization adapter
- Comprehensive unit and integration tests

**Planned Responsibility**
- Future storage backends (PostgreSQL, RocksDB, LMDB, DuckDB)
- Incremental update support

---

### kode-analysis

**Path:** `crates/analysis/`

**Current State**
- Parsing subsystem (Pipeline Stage 2) — transforms repository snapshots and source inventories into syntax tree inventories
- Fact Extraction subsystem (Pipeline Stage 3) — transforms syntax tree inventories into language-independent repository facts
- Language-specific parser implementations with registry-based dispatch
- Rust parser backed by Tree-sitter (fully encapsulated — no Tree-sitter types in public API)
- Pure transformations over immutable inputs (no filesystem I/O)
- Structured diagnostics model
- Language-specific extractors with registry-based dispatch
- Rust extractor supporting functions, structs, enums, traits, impls, type aliases, constants, statics, imports, and modules
- Stable, deterministic hash-based entity identifiers with source location evidence

**Planned Responsibility**
- Dependency analysis
- Impact analysis
- Cycle detection
- Architecture metrics
- Graph algorithms

---

### kode-query

**Path:** `crates/query/`

**Current State**
- Placeholder crate.

**Planned Responsibility**
- Intent resolution
- Graph traversal
- Evidence retrieval and verification
- Context assembly
- LLM interaction

---

### kode-common

**Path:** `crates/common/`

**Current State**
- Deterministic hashing infrastructure consumed by graph and storage crates
- Zero external dependencies — intentionally minimal to avoid circular coupling

**Planned Responsibility**
- Additional shared infrastructure as needed
- Framework-level utilities

---

### kode-app

**Path:** `services/app/`

**Current State**
- Application service layer for the scan pipeline
- Orchestrates repository discovery, source inventory loading, and parsing
- Public result types with scan statistics (timing, file counts, parse outcomes)
- Stable application API intended for CLI, MCP server, and third-party consumers
- Internal pipeline artifacts consumed and dropped before returning

**Planned Responsibility**
- Long-running service application
- Application lifecycle management
- Query pipeline orchestration

---

### kode-cli

**Path:** `tools/cli/`

**Current State**
- CLI argument parsing with subcommand routing
- Command hierarchy: scan, status, files, symbols, query, chat, cache, config, mcp
- **Implemented commands**: `scan` — runs full discovery/parsing pipeline; `status` — displays scan results; `files` — lists files with optional language filtering
- **Placeholder commands**: `symbols`, `query`, `chat`, `cache`, `config`, `mcp` — validate arguments, print placeholder message
- Presenter and formatter layers transforming domain types into display models
- Comprehensive test coverage for argument parsing, routing, help text, and global flags
- Global options: `--repo`, `--verbose`, `--quiet`, `--json`, `--no-color`

**Planned Responsibility**
- Wire remaining subcommands to application services
- Interactive REPL

---

## See Also

- [GUIDELINES.md](GUIDELINES.md) — Workspace conventions
- [Documentation index](README.md)
