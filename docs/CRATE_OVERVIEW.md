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

---

## Crate Reference

### kode-acquisition

**Path:** `crates/acquisition/`

**Current State**
- Repository domain model (canonical path, optional identity)
- `RepositoryDiscovery` orchestration (workspace → traversal → manifests → languages → snapshot)
- RepositorySnapshot with inventory types (File, Directory, Manifest, Language)
- Detector traits (WorkspaceDetector, ManifestDetector, LanguageDetector)
- Detector registries with first-match semantics
- Default detectors for Cargo workspaces, Cargo manifests, and extension-based language detection
- SnapshotBuilder for validated snapshot construction
- Workspace and Manifest domain models

**Planned Responsibility**
- Source code parsing (planned)
- Fact extraction (planned)
- Parser integration (planned)

---

### kode-graph

**Path:** `crates/graph/`

**Current State**
- Placeholder crate.

**Planned Responsibility**
- Knowledge Graph construction
- Node and relationship management
- Graph validation
- Graph traversal APIs

---

### kode-storage

**Path:** `crates/storage/`

**Current State**
- Placeholder crate.

**Planned Responsibility**
- Graph persistence
- Incremental indexing
- Cache management
- Graph revision tracking

---

### kode-analysis

**Path:** `crates/analysis/`

**Current State**
- Placeholder crate.

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
- Placeholder crate.

**Planned Responsibility**
- Shared infrastructure
- Framework-level utilities
- Lowest-level library crate

---

### kode-app

**Path:** `services/app/`

**Current State**
- Placeholder binary (empty `main`).

**Planned Responsibility**
- Long-running service application
- Application lifecycle management
- Service orchestration

---

### kode-cli

**Path:** `tools/cli/`

**Current State**
- Placeholder binary (empty `main`).

**Planned Responsibility**
- CLI argument parsing
- User-facing command execution
- Interactive REPL

---

## See Also

- [GUIDELINES.md](GUIDELINES.md) — Workspace conventions
- [Documentation index](README.md)
