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

---

## Next Release

- Call graph analysis — full resolution of function call relationships across languages
- Symbol search — fast, fuzzy symbol lookup across the Knowledge Graph
- Cache improvements — richer relationships in the incremental cache
- Graph export — export the Knowledge Graph to standard formats (GraphML, DOT, JSON)
- CLI refinements — improved output formatting, filtering, and pagination

---

## Near Term

- Architecture visualization — render the Knowledge Graph as interactive diagrams
- Impact analysis — determine what entities are affected by a given change
- Dead code detection — identify unused symbols with evidence
- Dependency analysis — compute dependency trees and detect cycles
- Architecture metrics — fan-in, fan-out, cohesion, and instability metrics
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
