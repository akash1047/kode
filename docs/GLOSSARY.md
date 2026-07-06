# Glossary

This document defines the core terminology used throughout **kode**.

## Repository

A software project rooted in a filesystem directory, containing source code, manifests, configuration, and documentation. The repository is the source of truth — everything in kode originates from it.

## RepositorySnapshot

An immutable structural snapshot of the repository. Includes file inventory, directory hierarchy, manifest locations, and language inventory. Contains no parsed source code. Produced by `RepositoryDiscovery`.

## RepositoryDiscovery

The orchestration layer that coordinates workspace detection, filesystem traversal, manifest discovery, language detection, and snapshot construction through composable detector registries.

## Workspace

A description of the repository's project structure — whether it is a single package, a multi-package workspace, or unstructured. Includes the root manifest location and member packages with their manifest paths.

## Manifest

A detected build configuration or package manifest file. Each manifest has a relative path within the repository and a manifest kind indicating its format.

## Inventory

A typed, ordered collection of discovered structural metadata. Inventory types include `FileInventory`, `DirectoryInventory`, `ManifestInventory`, and `LanguageInventory`. Inventories contain no source code or inferred knowledge.

## Detector

A stateless extension point that classifies some aspect of the repository. Detector types include `WorkspaceDetector`, `ManifestDetector`, and `LanguageDetector`. Identical inputs always produce identical outputs.

## Registry

A composite detector that coordinates multiple detector implementations in priority order using first-match semantics. Registry types include WorkspaceRegistry, ManifestRegistry, and LanguageRegistry.

## Syntax Tree

A language-specific abstract syntax tree (AST) produced by parsing a source file. Syntax trees are lossless — they preserve the complete syntactic structure required by later pipeline stages.

## Repository Fact

A language-independent unit of repository knowledge extracted from a syntax tree. Examples include declarations, definitions, imports, exports, and package metadata. Repository facts are objective — they describe what exists in the repository without interpretation.

## Knowledge Graph

The canonical representation of repository knowledge. A directed graph consisting of nodes (repository entities) and relationships (connections between entities), where every element is backed by evidence. The Knowledge Graph is deterministic, immutable once constructed, and language-independent.

## Node

An entity within the Knowledge Graph. Examples: Repository, Package, File, Function, Struct, Interface. Every node has a unique stable identifier and carries metadata appropriate for its kind.

## Relationship

A directed connection between two nodes in the Knowledge Graph. Every relationship has a source node, destination node, type, and evidence. Examples: `calls`, `imports`, `contains`, `implements`, `depends_on`.

## Evidence

A verifiable reference to a repository artifact that supports a node, relationship, or derived fact. Evidence is always a `path:line` (or range) location in the repository. Evidence is never produced by AI — it is always extracted deterministically.

## Graph Revision

An immutable version of the Knowledge Graph representing one complete repository state. Repository changes produce new graph revisions rather than mutating existing ones.

## Derived Fact

Repository knowledge computed from the Knowledge Graph rather than extracted directly from source code. Examples include dependency cycles, impact analysis results, and architecture metrics. Derived facts remain evidence-backed — the supporting evidence is the set of graph elements used during computation.

## Query Result

The output of the Query Engine. Contains repository entities, graph relationships, source locations, supporting evidence, and (where applicable) repository excerpts and derived analysis. Every query result is evidence-backed.

## Citation

A formatted evidence reference attached to an answer or result. Format: `path/to/file.rs:42`. Citations enable verification — every claim can be traced back to the repository.

---

## See Also

- [Documentation index](README.md) — All documents
