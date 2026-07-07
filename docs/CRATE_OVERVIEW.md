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
- Repository domain model (canonical path, optional identity)
- `RepositoryDiscovery` orchestration (workspace → traversal → manifests → languages → snapshot)
- RepositorySnapshot with inventory types (File, Directory, Manifest, Language)
- Detector traits (WorkspaceDetector, ManifestDetector, LanguageDetector)
- Detector registries with first-match semantics
- Default detectors for Cargo workspaces, Cargo manifests, and extension-based language detection
- SnapshotBuilder for validated snapshot construction
- Workspace and Manifest domain models

**Planned Responsibility**
- (none — downstream processing is handled by kode-analysis)

---

### kode-graph

**Path:** `crates/graph/`

**Current State**
- Stage 4 (Graph Construction) fully implemented
- `KnowledgeGraph` — immutable, validated, traversable in-memory graph
- `GraphNodeId` — dual identity model: `Structural(StructuralNodeId)` for synthetic nodes, `Entity(EntityId)` for extracted entities
- `GraphEvidence` — dual evidence model: `Source(Evidence)` for parser-produced evidence, `Structural(StructuralEvidence)` for synthetic graph elements
- `StructuralEvidence` — typed enum with `Repository`, `Workspace`, and `File` variants
- `Node` — graph node with kind, name, metadata, and evidence
- `Relationship` — directed edge with kind, metadata, and evidence
- `GraphBuilder` — decomposed into focused sub-modules; performs **zero repository discovery**
- `RepositoryContext` — sole owner of temporary repository metadata derivation (external to builder)
- `GraphBuildState` — single mutable state object during construction
- `StructuralLookup` — pre-computed structural IDs (generated once, consumed by relationship building)
- `GraphValidator` — topology validation (structural roots, orphan detection, duplicate safety net); constructor invariants are enforced at the type level
- Integration tests covering structure, determinism, evidence, traversal, edge cases, and structural separation

**Ownership Boundaries**
- `RepositoryContext` is constructed externally and passed to `GraphBuilder::build`
- Structural IDs are generated exactly once and cached in `StructuralLookup`
- All temporary repository heuristics live in exactly one file (`builder/context.rs`)

**Public API**
- `GraphBuilder::build(&RepositoryFacts, &RepositoryContext) -> Result<KnowledgeGraph, Vec<ValidationError>>`
- `RepositoryContext::from_facts(&RepositoryFacts) -> Self` (temporary — Stage 5 replaces this)
- `RepositoryContext::new(root_path, workspace_name, source_files) -> Self` (canonical constructor)
- `KnowledgeGraph::node_by_id(&GraphNodeId) -> Option<&Node>`
- `KnowledgeGraph::nodes()`, `relationships()`, `node_count()`, `relationship_count()`
- `KnowledgeGraph::nodes_by_kind(NodeKind) -> impl Iterator<Item = &Node>`
- `KnowledgeGraph::outgoing(&GraphNodeId)`, `incoming(&GraphNodeId)`
- `GraphNodeId::Structural(StructuralNodeId)` / `GraphNodeId::Entity(EntityId)`
- `GraphEvidence::Source(Evidence)` / `GraphEvidence::Structural(StructuralEvidence)`
- `StructuralEvidence` — typed structural evidence: `Repository { root: PathBuf }`, `Workspace { name }`, `File { path }`
- `StructuralNodeId::from_parts(StructuralNodeKind, path, name)` — deterministic identity (FNV-1a hashing from `kode_common::hash`)

**Shared Constants**
- `DEFAULT_WORKSPACE_NAME` — `"default"`
- `REPOSITORY_HASH_NAME` — `"repo"`
- `REPOSITORY_NODE_NAME` — `"repository"`

**Planned Responsibility**
- Stage 5: Replace `RepositoryContext::from_facts` with Acquisition-provided metadata
- Future: additional index structures (name, qualified name, file lookup)

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
- Parsing subsystem (Pipeline Stage 2) — transforms `RepositorySnapshot` and `SourceInventory` into `SyntaxTreeInventory`
- Fact Extraction subsystem (Pipeline Stage 3) — transforms `SyntaxTreeInventory` into `RepositoryFacts`
- `Parser` trait for language-specific parser implementations (no `can_parse()` — selection is language-keyed)
- `ParserRegistry` with language-keyed dispatch (separate dispatcher, not a parser implementation)
- `RustParser` — Tree-sitter-backed Rust parser
- `ParsingOrchestrator` — pure transformation over immutable inputs (no filesystem I/O)
- `SourceInventory` — immutable source text storage (`Arc<str>`) loaded before parsing
- `SyntaxTree`, `SyntaxTreeInventory`, `FileParseOutcome` — immutable domain artifacts
- `SyntaxTreeInventory` with indexed lookup (`HashMap`) and deterministic iteration
- Structured diagnostics model (`Diagnostic`, `Severity`)
- Error taxonomy covering parser initialization and unsupported languages
- Tree-sitter fully encapsulated behind `pub(crate)` adapter — no Tree-sitter types in public API
- Expanded parser metadata (`grammar_version`, `backend_id`)
- `Extractor` trait for language-specific extractors
- `ExtractorRegistry` with language-keyed dispatch
- `RustExtractor` — Tree-sitter-backed Rust extractor for functions, structs, enums, traits, impls, type aliases, constants, statics, imports, and modules
- `ExtractionOrchestrator` — pure transformation over immutable inputs (no filesystem I/O)
- `RepositoryFacts` — immutable domain artifact containing all extracted entities
- `EntityId` — stable, deterministic hash-based entity identifiers
- `Evidence` — source location evidence attached to every entity
- Entity types: `ModuleFact`, `FunctionFact`, `StructFact`, `EnumFact`, `TraitFact`, `ImplBlockFact`, `TypeAliasFact`, `ConstantFact`, `StaticFact`, `ImportFact`
- Extraction diagnostics model (`ExtractionDiagnostic`, `Severity`)

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
