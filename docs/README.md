# Documentation

This index describes every document in the **kode** documentation set.

Architecture documents describe the intended system. The Acquisition
subsystem is implemented — see [ACQUISITION.md](ACQUISITION.md) for the
current state. Remaining subsystems are planned and follow the boundaries
established here.

Contributors should read the architecture documents to understand the
design intent, then refer to the contributor documents for workspace
conventions and crate responsibilities.

The CLI specification (`CLI_SPEC.md`) is the implementation contract for
the command-line interface. It is auto-verified against the generated
clap help output.

---

## Reading Order

New contributors should read in this order:

```text
README
  ↓
DESIGN
  ↓
ARCHITECTURE
  ↓
CLI_SPEC              (implementation contract)
  ↓
PIPELINE
  ↓
ACQUISITION
  ↓
KNOWLEDGE_GRAPH
  ↓
STORAGE
  ↓
ANALYSIS
  ↓
QUERY_ENGINE
  ↓
MCP
```

---

## Document Map

### Architecture Documents

| Document | Description |
|----------|-------------|
| [DESIGN.md](../DESIGN.md) | System design, architectural principles, and invariants |
| [ARCHITECTURE.md](ARCHITECTURE.md) | High-level architecture and subsystem interaction |
| [PIPELINE.md](PIPELINE.md) | Repository processing pipeline — from source code to structured knowledge |
| [ACQUISITION.md](ACQUISITION.md) | Acquisition subsystem — repository discovery, snapshot construction, detector architecture (parsing and fact extraction planned) |
| [KNOWLEDGE_GRAPH.md](KNOWLEDGE_GRAPH.md) | Knowledge Graph specification — nodes, relationships, and evidence |
| [STORAGE.md](STORAGE.md) | Persistence, caching, and incremental updates |
| [QUERY_ENGINE.md](QUERY_ENGINE.md) | Query execution — transforming knowledge into evidence-backed answers |
| [ANALYSIS.md](ANALYSIS.md) | Analysis engine — derived facts and graph algorithms |
| [EVIDENCE_MODEL.md](EVIDENCE_MODEL.md) | Evidence guarantees and citation model |
| [MCP.md](MCP.md) | Model Context Protocol integration |

### Contributor Documents

| Document | Description |
|----------|-------------|
| [GUIDELINES.md](GUIDELINES.md) | Workspace conventions, crate design, and development workflow |
| [CRATE_OVERVIEW.md](CRATE_OVERVIEW.md) | Crate responsibilities and dependency graph |
| [CLI_SPEC.md](CLI_SPEC.md) | CLI implementation contract (matches generated help) |
| [CLI.md](CLI.md) | CLI command reference and examples |
| [ROADMAP.md](ROADMAP.md) | Planned evolution and milestones |
| [GLOSSARY.md](GLOSSARY.md) | Terminology reference |

---

## See Also

- [README.md](../README.md) — Project overview
- [CLI.md](CLI.md) — Command reference
- [GUIDELINES.md](GUIDELINES.md) — Contributor workflow
