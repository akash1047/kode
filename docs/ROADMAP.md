# Roadmap

This document describes the evolution of **kode** organized by milestone rather than timeline.

---

## Completed Milestones

### Milestone 1 — CLI Foundation

| | |
|---|---|
| Status | ✓ Complete |
| Delivered | Public CLI contract, command hierarchy, argument parsing, help generation, command dispatch, placeholder handlers, parser validation |
| Next | Repository discovery and indexing |

### Milestone 2 — Acquisition & Analysis

| | |
|---|---|
| Status | ✓ Complete |
| Delivered | Stage 1-3 pipeline: repository discovery, parsing, fact extraction. Immutable snapshot, syntax tree inventory, and repository facts artifacts. Stable entity identification, source location evidence, extractors for all Rust entity kinds. |

### Milestone 3 — Knowledge Graph

| | |
|---|---|
| Status | ✓ Complete |
| Delivered | Stage 4: Graph construction and validation. Deterministic graph construction with structural/entity identity separation, evidence-backed nodes and relationships, decomposed builder architecture, comprehensive validation invariant checks. Deterministic graph serialization with round-trip fidelity. |

### Milestone 4 — Storage

| | |
|---|---|
| Status | ✓ Complete |
| Delivered | Stage 5: Graph persistence. Revisioned persistence with a backend abstraction, repository-scoped storage, schema versioning, immutable graph revisions, and cache state tracking. Content-addressed deterministic persistence with transactional guarantees. Graph serialization delegated to the graph crate's deterministic format. |

---

## Current Milestone

### Milestone 5 — Analysis & Query

| | |
|---|---|
| Status | ✓ Complete |
| Delivered | Pipeline wiring (extraction→graph→storage), query engine with symbol search and evidence verification, call graph relationship kind, graph export (DOT, GraphML), CLI refinements (symbols, query, cache status/clear, config init/get/set) |

---

## Current Milestone

### Milestone 6 — Advanced Analysis

| | |
|---|---|
| Status | 🔄 In Progress |
| Target | Cache improvements, MCP server, deeper graph algorithms |

Planned deliverables:

- Cache improvements — richer relationships in the incremental cache
- MCP server — expose query engine through the Model Context Protocol
- Architecture visualization — render the Knowledge Graph as interactive diagrams
- Impact analysis — determine what entities are affected by a given change

## Near Term

- Architecture metrics — fan-in, fan-out, cohesion, and instability metrics
- Dead code detection — identify unused symbols with evidence
- Dependency analysis — compute dependency trees and detect cycles
- Multi-repository indexing — index and query across related repositories
- LSP integration — expose repository knowledge through the Language Server Protocol

---

## Future

- IDE integrations — VS Code extension, JetBrains plugin, and editor-agnostic interfaces
- Web UI — browser-based repository exploration and querying
- GraphQL API — structured query API for programmatic access
- Git history analysis — track symbol evolution across commits
- Code ownership analysis — map repository entities to contributors
- Test coverage integration — associate tests with tested entities
- CI integration — run analysis as part of continuous integration
- Security scanning — detect vulnerable dependency patterns

---

## Research

- Runtime trace integration — associate runtime behavior with graph entities
- Performance profiling — link profile data to source locations
- Code generation tracking — identify generated code and its generators
- Natural language query parsing — map questions directly to graph traversals

---

## Non-Goals

- Code generation — kode will not generate code. Its purpose is understanding existing code.
- AI training — kode will not use repository data to train or fine-tune models.
- Cloud dependency — kode remains local-first. Cloud features are always optional.
- Real-time collaboration — kode is a single-user tool. Multi-user features are not planned.
- Proprietary lock-in — all formats and protocols remain open and documented.

---

## See Also

- [DESIGN.md](../DESIGN.md) — System design
- [ARCHITECTURE.md](ARCHITECTURE.md) — High-level architecture
- [GLOSSARY.md](GLOSSARY.md) — Terminology
- [Documentation index](README.md)
