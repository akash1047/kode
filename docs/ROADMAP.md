# Roadmap

This document describes the evolution of **kode** organized by milestone rather than timeline.

---

## Completed Milestones

### Milestone 1 — CLI Foundation

| | |
|---|---|
| Status | ✓ Complete |
| Delivered | Public CLI contract, command hierarchy, argument parsing, help generation, command dispatch |

### Milestone 2 — Acquisition & Analysis

| | |
|---|---|
| Status | ✓ Complete |
| Delivered | Stage 1-3 pipeline: repository discovery, parsing, fact extraction. Immutable snapshot, syntax tree inventory, and repository facts. Rust entity kinds + call sites. |

### Milestone 3 — Knowledge Graph

| | |
|---|---|
| Status | ✓ Complete |
| Delivered | Stage 4: Graph construction and validation, `Calls` edges, deterministic serialization |

### Milestone 4 — Storage

| | |
|---|---|
| Status | ✓ Complete |
| Delivered | Stage 5: SQLite revisions, backend abstraction, content fingerprint for incremental skip |

### Milestone 5 — Analysis & Query

| | |
|---|---|
| Status | ✓ Complete |
| Delivered | Pipeline wiring, query engine, export (DOT/GraphML), symbols/query/cache/config CLI |

### Milestone 5b — Interfaces & Chat

| | |
|---|---|
| Status | ✓ Complete |
| Delivered | MCP server, agent TUI chat (tools + symbol index), graph export, cache-backed status/files, `.kodeignore` |

### Milestone 6 — Advanced Analysis

| | |
|---|---|
| Status | ✓ Complete (product scope) |
| Delivered | Rust call-site extraction, `Calls` edges, find_callers / find_callees / impact_analysis, architecture metrics (fan-in/out, instability), dead-code heuristic, call-graph cycles, intent router, stronger evidence verification, whole-repo fingerprint skip, **per-file incremental reparse** (facts cache + content hashes), **`Imports` edges**, **Python** parse/extract MVP |

Remaining beyond M6:

- `--watch` / parallel `--threads`
- Deeper multi-language support (TS/Go/…)
- Stronger import resolution (cargo metadata)

---

## Near Term

- ~~Per-file incremental indexing~~ (done)
- ~~Python extractor MVP~~ (done)
- ~~Import edges from `use` paths~~ (done, name-based)
- Stronger multi-language (TypeScript, Go, …)
- `--watch` and parallel scan workers
- Multi-repository indexing
- LSP integration

---

## Future

- IDE integrations — VS Code extension, JetBrains plugin
- Web UI — browser-based exploration
- GraphQL API
- Git history analysis
- Code ownership analysis
- Test coverage integration
- CI product integration
- Security scanning patterns

---

## Research

- Runtime trace integration
- Performance profiling linkage
- Code generation tracking
- Natural language query parsing (ML)

---

## Non-Goals

- Code generation — kode will not generate code
- AI training on user repositories
- Cloud dependency as a requirement (local-first)
- Real-time multi-user collaboration
- Proprietary lock-in — formats and protocols stay open

---

## See Also

- [DESIGN.md](../DESIGN.md) — System design
- [ARCHITECTURE.md](ARCHITECTURE.md) — High-level architecture
- [GLOSSARY.md](GLOSSARY.md) — Terminology
- [Documentation index](README.md)
