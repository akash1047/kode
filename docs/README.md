# Documentation

This index describes every document in the **kode** documentation set.

Architecture documents describe the intended system. Acquisition, Parsing,
Fact Extraction, Knowledge Graph, Storage, Query Engine, MCP, and Chat are
implemented for the Rust-first product path — see subsystem docs and
[ROADMAP.md](ROADMAP.md). Near-term gaps: per-file incremental reparse,
multi-language extractors, `--watch` / parallel scan.

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
ANALYSIS
  ↓
KNOWLEDGE_GRAPH
  ↓
STORAGE
  ↓
QUERY_ENGINE
  ↓
MCP
```

---

## Document Structure

Architecture documents follow a consistent structure:

1. **Purpose** — what the subsystem exists to do
2. **Responsibilities** — what the subsystem owns and does not own
3. **Position in Architecture** — how the subsystem fits into the pipeline
4. **Architecture** — conceptual model, contracts, and design (backend-neutral)
5. **Current Implementation** (optional) — notes on the existing codebase, may
   evolve independently of architecture
6. **Future Evolution** — planned capabilities and extension points
7. **Design Constraints** — invariants every implementation must satisfy

Architecture documents intentionally avoid duplicating Rust APIs.
**rustdoc** is the authoritative API reference.
Architecture documents describe concepts, ownership, and boundaries — not
types, traits, or implementation details.

---

## Document Map

### Architecture Documents

| Document | Description |
|----------|-------------|
| [DESIGN.md](../DESIGN.md) | System design, architectural principles, and invariants |
| [ARCHITECTURE.md](ARCHITECTURE.md) | High-level architecture and subsystem interaction |
| [PIPELINE.md](PIPELINE.md) | Repository processing pipeline — from source code to structured knowledge |
| [ACQUISITION.md](ACQUISITION.md) | Acquisition subsystem — repository discovery, snapshot construction, detector architecture |
| [KNOWLEDGE_GRAPH.md](KNOWLEDGE_GRAPH.md) | Knowledge Graph specification — nodes, relationships, and evidence |
| [STORAGE.md](STORAGE.md) | Storage architecture — persistence, revision model, and backend abstraction |
| [QUERY_ENGINE.md](QUERY_ENGINE.md) | Query execution — transforming knowledge into evidence-backed answers |
| [ANALYSIS.md](ANALYSIS.md) | Analysis engine — Parsing (Stage 2), Fact Extraction (Stage 3), and future graph algorithms |
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
