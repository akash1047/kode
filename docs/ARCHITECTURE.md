# Architecture

This document describes the high-level architecture of **kode**.

It explains how the major subsystems interact, the responsibilities of each
component, and the architectural boundaries between them.

Detailed specifications for each subsystem are documented separately.

---

## Terminology

See [GLOSSARY.md](GLOSSARY.md) for definitions of all core terms.

---

## Overview

kode is organized as a deterministic pipeline centered around a
Knowledge Graph.

Every subsystem either:

- contributes information to the Knowledge Graph, or
- consumes information from it.

The graph is the architectural center of the system.

```mermaid
flowchart LR

    Repository --> Acquisition

    Acquisition --> KnowledgeGraph["Knowledge Graph"]

    KnowledgeGraph --> Storage
    KnowledgeGraph --> Analysis
    KnowledgeGraph --> QueryEngine["Query Engine"]

    Analysis --> QueryEngine

    QueryEngine --> Interfaces

    Interfaces --> CLI
    Interfaces --> MCP
    Interfaces --> LLM
```

Each stage has a single responsibility.

Information flows in one direction.

---

## Architectural Principles

The architecture follows several fundamental principles defined in [DESIGN.md](../DESIGN.md#architecture-principles) — Repository First, Deterministic, Evidence First, Separation of Concerns, and Incremental processing.

These principles are documented fully in DESIGN.md. All subsystems inherit them.

### Pipeline Ownership Note

The pipeline ownership table in [PIPELINE.md](PIPELINE.md) assigns Parsing (Stage 2) and Fact Extraction (Stage 3) to the Analysis subsystem, implemented in the `kode-analysis` crate. Graph Construction (Stage 4) and Validation are owned by the `kode-graph` crate. Persistence (Stage 5) is owned by the `kode-storage` crate. Each subsystem owns one responsibility and ownership never overlaps.

---

## System Context

```mermaid
flowchart TD

    User

    Agent["AI Agent"]

    Repository

    User --> CLI

    Agent --> MCP

    CLI --> Repository

    MCP --> Repository
```

Both humans and AI agents interact with the same repository model.

There are no separate execution paths.

---

## Component Overview

```mermaid
flowchart LR

    Acquisition

    Graph["Knowledge Graph"]

    Storage

    Analysis

    Query["Query Engine"]

    Interfaces

    Acquisition --> Graph
    Graph --> Storage
    Graph --> Analysis
    Analysis --> Query
    Graph --> Query
    Query --> Interfaces
```

---

## Acquisition Layer

Responsible for discovering repository structure and producing an
immutable snapshot.

Responsibilities include

- repository discovery
- workspace detection
- filesystem traversal
- manifest discovery
- language detection
- snapshot construction

The Acquisition boundary artifact is the **RepositorySnapshot** — an
immutable structural snapshot of the repository containing file inventory,
directory hierarchy, manifest locations, and language inventory.

Acquisition ends at RepositorySnapshot. Downstream processing (Parsing,
Fact Extraction) is handled by the Analysis subsystem.

Output:

RepositorySnapshot.

See:

- ACQUISITION.md
- PIPELINE.md

---

## Knowledge Graph Layer

Responsible for representing repository knowledge.

Responsibilities include

- graph construction
- node management
- relationship management
- graph validation
- graph queries (low-level traversal)

The graph is immutable once constructed.

See:

- KNOWLEDGE_GRAPH.md

---

## Storage Layer

Responsible for persistence.

Responsibilities include

- graph persistence
- cache management
- graph revisions
- incremental indexing

Storage never performs analysis.

See:

- STORAGE.md

---

## Analysis Layer

Responsible for deriving information from the repository and graph.

Responsibilities include

- **Parsing (Stage 2)** — transforms `RepositorySnapshot` and `SourceInventory` into `SyntaxTreeInventory` using language-specific parsers with an encapsulated syntax backend
- **Fact Extraction (Stage 3)** — transforms `SyntaxTreeInventory` into `RepositoryFacts` using language-specific extractors that walk syntax trees and produce entities with evidence
- dependency analysis (planned)
- impact analysis (planned)
- cycle detection (planned)
- architecture metrics (planned)
- dead code detection (planned)

The Analysis boundary includes:

```
RepositorySnapshot
        ↓
SourceInventory
        ↓
SyntaxTreeInventory
        ↓
RepositoryFacts
```

The Analysis boundary artifacts are the **SyntaxTreeInventory** (Stage 2) and
**RepositoryFacts** (Stage 3) — immutable collections of parse outcomes and
extracted entities respectively.

Parsing is the entry point to the Analysis layer and the Stage 2 producer.
Fact Extraction is the Stage 3 transformation producing the semantic model
consumed by the Knowledge Graph.

See:

- ANALYSIS.md
- PIPELINE.md

---

## Query Engine

Transforms repository knowledge into evidence-backed answers.

Responsibilities include

- intent resolution
- graph traversal
- evidence retrieval and verification
- context assembly

The Query Engine consumes the Knowledge Graph for traversal and Analysis results for derived facts. It never builds repository knowledge.

Consumers should never manipulate graph internals directly.

See:

- QUERY_ENGINE.md

---

## Interface Layer

Interfaces expose functionality to external consumers.

Examples include

- CLI
- MCP
- future HTTP API
- future IDE integrations

Interfaces never perform parsing.

Interfaces never build repository knowledge.

They consume existing graph data.

---

## Layer Dependencies

Dependencies always flow downward.

```mermaid
flowchart TD

Interfaces

QueryEngine["Query Engine"]

Analysis

Storage

KnowledgeGraph["Knowledge Graph"]

Acquisition

Utilities

Interfaces --> QueryEngine
QueryEngine --> Analysis
QueryEngine --> KnowledgeGraph
Storage --> Analysis
Storage --> KnowledgeGraph
KnowledgeGraph --> Acquisition
Acquisition --> Utilities
```

Circular dependencies are prohibited.

---

## Data Flow

Repository processing follows a deterministic lifecycle.

```mermaid
sequenceDiagram

participant Repo as Repository
participant Acquisition
participant Snapshot as "RepositorySnapshot"
participant Graph as "Knowledge Graph"
participant Storage
participant Analysis
participant Query as "Query Engine"
participant LLM

Repo->>Acquisition: Discover structure

Acquisition->>Snapshot: Produce

Note over Snapshot: Acquisition boundary artifact

Snapshot->>Graph: Build graph (planned)

Graph->>Storage: Persist

Graph->>Analysis: Derive facts

Analysis->>Query: Derived facts

LLM->>Query: Ask question

Query->>Graph: Traverse graph

Graph-->>Query: Matching entities

Query->>Analysis: Retrieve derived facts

Analysis-->>Query: Analysis results

Query->>Repo: Read source files

Repo-->>Query: Evidence

Query-->>LLM: Context + citations
```

The graph accelerates discovery (see [QUERY_ENGINE.md](QUERY_ENGINE.md)). The repository provides verification.

---

## Extension Points

The architecture intentionally supports extension.

Examples include

- new language parsers
- new manifest formats
- new graph algorithms
- new exporters
- new storage backends
- new interface adapters

These extensions should not require architectural changes.

---

## Future Interfaces

The current interfaces are

- CLI
- MCP

Future interfaces may include

- Language Server Protocol (LSP)
- VS Code extension
- JetBrains plugin
- Web UI
- GraphQL API

These interfaces will consume the same Query Engine.

---

## Architectural Invariants

The architectural invariants defined in [DESIGN.md](../DESIGN.md#architectural-invariants) apply to all subsystems. Every implementation must preserve them.

---

## See Also

- [DESIGN.md](../DESIGN.md) — System design and invariants
- [Documentation index](README.md) — All documents

