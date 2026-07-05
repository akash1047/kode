# Repository Processing Pipeline

This document describes how **kode** transforms a software repository into
structured, deterministic, and queryable knowledge.

The pipeline is the backbone of repository understanding.

Given the same repository contents, the pipeline will always produce the
same Knowledge Graph and the same Repository Facts.

Repository understanding is therefore reproducible, incremental, and
independent of AI models.

---

## Purpose

The repository processing pipeline exists to transform raw source code into
structured repository knowledge that can be efficiently queried, analyzed,
and consumed by humans and AI agents.

The pipeline provides:

- deterministic repository analysis
- language-independent processing
- incremental execution
- evidence preservation
- reproducible outputs

Every stage has a single responsibility.

Every stage produces an immutable artifact consumed by the next stage.

---

## Design Principles

The pipeline follows the architectural principles defined in [DESIGN.md](../DESIGN.md#architecture-principles): deterministic, incremental, evidence-first, immutable artifacts, and separation of concerns.

These principles are documented fully in DESIGN.md.

---

## Processing Lifecycle

Repository understanding follows a deterministic sequence.

```mermaid
flowchart LR

    Repository

    --> Discovery

    --> RepositorySnapshot

    --> Parsing

    --> SyntaxTrees

    --> FactExtraction

    --> RepositoryFacts

    --> GraphConstruction

    --> KnowledgeGraph["Knowledge Graph"]

    --> Validation

    --> ValidatedGraph["Validated Graph"]

    --> Persistence

    --> GraphRevision["Graph Revision"]

    --> Analysis

    --> DerivedFacts["Derived Facts"]

    --> QueryEngine["Query Engine"]

    --> EvidenceBackedAnswer["Evidence-backed Answer"]
```

Each stage transforms one artifact into another.

Earlier stages discover facts.

Later stages organize, analyze, and present those facts.

---

## Pipeline Summary

| Stage | Input | Output | Owner |
|---------|--------|---------|---------|
| Repository Discovery | Repository | Repository Snapshot | Acquisition |
| Parsing | Repository Snapshot | Syntax Trees | Acquisition |
| Fact Extraction | Syntax Trees | Repository Facts | Acquisition |
| Graph Construction | Repository Facts | Knowledge Graph | Graph |
| Graph Validation | Knowledge Graph | Validated Graph | Graph |
| Persistence | Validated Graph | Graph Revision | Storage |
| Analysis | Graph Revision | Derived Facts | Analysis |
| Query Execution | Graph Revision + Repository | Evidence-backed Results | Query Engine |

Each stage owns exactly one transformation.

No stage bypasses another.

---

## Pipeline Artifacts

The pipeline progressively transforms repository information into higher
levels of abstraction.

```mermaid
flowchart TD

A["Repository"]

B["Repository Snapshot"]

C["Syntax Trees"]

D["Repository Facts"]

E["Knowledge Graph"]

F["Validated Graph"]

G["Graph Revision"]

H["Derived Facts"]

I["Evidence-backed Results"]

A --> B
B --> C
C --> D
D --> E
E --> F
F --> G
G --> H
H --> I
```

Each artifact is immutable.

Each artifact has a single producer.

Each artifact may have many consumers.

---

## Stage 1 — Repository Discovery

See [ACQUISITION.md](ACQUISITION.md) for the Acquisition subsystem specification.

### Purpose

Discover the repository and produce a deterministic snapshot of its
structure.

This stage establishes the processing boundary for every later stage.

No source code is parsed.

No repository knowledge is inferred.

---

### Input

Repository root.

---

### Output

Repository Snapshot.

The snapshot contains only repository metadata.

---

### Responsibilities

Repository Discovery is responsible for:

- locating the repository root
- detecting workspace boundaries
- enumerating repository files
- respecting `.gitignore`
- identifying supported languages
- locating manifests
- locating build configuration
- locating documentation
- collecting filesystem metadata

---

### Produced Artifact

Repository Snapshot.

Example contents include:

- repository root
- workspace members
- directory hierarchy
- file inventory
- manifest locations
- language inventory

---

### Invariants

This stage must satisfy the following constraints.

- deterministic
- read-only
- filesystem only
- no parsing
- no graph construction
- no repository interpretation

---

### Failure Modes

Repository discovery may fail when:

- the repository cannot be located
- filesystem permissions prevent traversal
- the workspace layout is invalid
- manifests cannot be resolved

Failure terminates the pipeline.

Partial Repository Snapshots are never emitted.

---

## Stage 2 — Parsing

See [ACQUISITION.md](ACQUISITION.md) for the parser abstraction specification.

### Purpose

Transform repository source files into language-specific syntax trees.

Parsing extracts syntax without interpreting repository semantics.

The parser understands language grammar.

It does not understand architecture.

---

### Input

Repository Snapshot.

---

### Output

Syntax Trees.

Each supported language produces its own abstract syntax tree (AST).

---

### Responsibilities

Parsing is responsible for:

- selecting the correct parser
- parsing source files
- reporting syntax errors
- preserving source locations
- exposing language-specific syntax

Supported parsers operate independently.

Adding a new language parser should not require changes elsewhere in the
pipeline.

---

### Produced Artifact

Syntax Trees.

Each syntax tree represents one source file.

Syntax trees preserve the complete syntactic structure required by later
pipeline stages.

---

### Invariants

Parsing must satisfy the following constraints.

- deterministic
- language specific
- lossless
- no repository interpretation
- no graph construction
- no dependency analysis

---

### Failure Modes

Parsing may fail due to:

- malformed source code
- unsupported language versions
- parser implementation errors

Parser failures should identify the affected file without corrupting the
remaining pipeline.

## Stage 3 — Fact Extraction

See [ACQUISITION.md](ACQUISITION.md) for the Acquisition subsystem specification.

### Purpose

Transform language-specific syntax trees into language-independent repository
facts.

Fact Extraction is the bridge between parsing and repository understanding.

At the end of this stage, all extracted information has a common
representation regardless of programming language.

No relationships are inferred.

No analysis is performed.

Only objective repository facts are produced.

---

### Input

Syntax Trees.

---

### Output

Repository Facts.

---

### Responsibilities

Fact Extraction is responsible for:

- identifying repository entities
- extracting symbols
- extracting declarations
- extracting definitions
- extracting imports
- extracting exports
- extracting package information
- extracting manifests
- preserving source locations
- attaching evidence

The extractor normalizes language-specific constructs into a common model.

---

### Produced Artifact

Repository Facts.

Examples include:

- repository
- workspace
- package
- module
- file
- function
- method
- struct
- enum
- trait
- interface
- class
- variable
- constant
- macro

Repository Facts remain language independent.

---

### Invariants

Fact Extraction must satisfy the following constraints.

- deterministic
- language independent
- evidence preserving
- no graph construction
- no dependency analysis
- no interpretation

---

### Failure Modes

Fact Extraction may fail when:

- parser output is invalid
- mandatory metadata is missing
- unsupported syntax is encountered

Extraction failures terminate processing for the affected repository revision.

---

## Stage 4 — Graph Construction

### Purpose

Transform repository facts into the canonical Knowledge Graph.

Graph Construction organizes isolated facts into a connected repository
representation.

Every repository entity becomes a node.

Every deterministic relationship becomes an edge.

---

### Input

Repository Facts.

---

### Output

Knowledge Graph.

---

### Responsibilities

Graph Construction is responsible for:

- creating nodes
- creating relationships
- assigning identifiers
- attaching metadata
- attaching evidence
- preserving deterministic ordering

The graph becomes the canonical representation of repository knowledge.

---

### Produced Artifact

Knowledge Graph.

The graph contains:

- nodes
- relationships
- evidence
- metadata

Every graph element originates from repository facts.

---

### Invariants

Graph Construction must satisfy:

- deterministic
- immutable
- language independent
- evidence backed
- reproducible

Graph construction never performs architectural interpretation.

---

### Failure Modes

Graph Construction may fail when:

- repository facts are inconsistent
- duplicate identifiers are generated
- invalid relationships are detected

Invalid graphs are discarded.

---

## Stage 5 — Graph Validation

### Purpose

Ensure the constructed graph satisfies every architectural invariant before it
becomes available to consumers.

Validation protects downstream systems from malformed repository knowledge.

---

### Input

Knowledge Graph.

---

### Output

Validated Graph.

---

### Responsibilities

Validation verifies:

- node uniqueness
- relationship integrity
- evidence completeness
- metadata completeness
- graph consistency
- identifier stability

Only validated graphs may proceed further.

---

### Produced Artifact

Validated Graph.

This graph is considered the canonical repository model.

---

### Invariants

Validation never modifies repository knowledge.

It either:

- accepts the graph, or
- rejects the graph.

---

### Failure Modes

Validation fails when:

- duplicate nodes exist
- relationships reference missing nodes
- evidence is missing
- graph invariants are violated

Rejected graphs never reach persistence.

---

## Stage 6 — Persistence

### Purpose

Persist the validated graph for future executions.

Persistence makes repository processing incremental.

Subsequent executions should reuse existing repository knowledge whenever
possible.

---

### Input

Validated Graph.

---

### Output

Graph Revision.

---

### Responsibilities

Persistence is responsible for:

- storing graph nodes
- storing relationships
- storing evidence
- storing repository metadata
- creating graph revisions
- updating cache metadata

Persistence never changes repository knowledge.

It only stores it.

---

### Produced Artifact

Graph Revision.

Each revision represents one complete repository state.

Graph revisions are immutable.

---

### Invariants

Persistence must satisfy:

- transactional updates
- deterministic storage
- immutable revisions
- rollback on failure

---

### Failure Modes

Persistence may fail because of:

- storage corruption
- insufficient disk space
- transaction failures
- schema incompatibility

Failed persistence never exposes partial graph revisions.

## Stage 7 — Analysis

### Purpose

Derive higher-level repository knowledge from the persisted Knowledge Graph.

Unlike previous stages, Analysis does not discover new repository facts.

Instead, it interprets existing graph relationships to answer architectural
questions and compute repository metrics.

Analysis is entirely deterministic.

---

### Input

Graph Revision.

---

### Output

Derived Facts.

---

### Responsibilities

Analysis is responsible for:

- dependency analysis
- impact analysis
- cycle detection
- reachability analysis
- architecture metrics
- graph metrics
- dead code detection
- dependency visualization
- repository summaries

Analysis never modifies the Knowledge Graph.

It is a pure consumer of graph data.

---

### Produced Artifact

Derived Facts.

Examples include:

- dependency trees
- dependency cycles
- strongly connected components
- fan-in metrics
- fan-out metrics
- architecture summaries
- repository statistics
- impact reports

Derived Facts remain evidence-backed.

Every result must remain traceable to graph elements.

---

### Invariants

Analysis must satisfy the following constraints.

- deterministic
- read-only
- reproducible
- graph preserving
- evidence preserving

Analysis must never introduce repository knowledge unsupported by the graph.

---

### Failure Modes

Analysis may fail due to:

- invalid graph revisions
- unsupported algorithms
- resource limitations

Analysis failures never invalidate the underlying graph revision.

---

## Stage 8 — Query Execution

### Purpose

Transform repository knowledge into evidence-backed answers.

Query Execution is the boundary between repository understanding and user
interaction.

It retrieves relevant graph elements, verifies evidence against the
repository, and produces structured results for consumers.

---

### Input

- Graph Revision
- Repository
- User Query

---

### Output

Evidence-backed Results.

---

### Responsibilities

Query Execution is responsible for:

- interpreting user intent
- traversing the Knowledge Graph
- locating relevant entities
- retrieving supporting evidence
- verifying evidence against repository files
- preparing context for consumers
- attaching citations

The Query Engine does not create repository knowledge.

It retrieves existing knowledge.

---

### Produced Artifact

Evidence-backed Results.

These results may contain:

- repository entities
- graph relationships
- source locations
- supporting evidence
- repository excerpts
- derived analysis

Consumers decide how these results are presented.

---

### Invariants

Query Execution must satisfy:

- deterministic graph traversal
- repository verification
- evidence preservation
- citation attachment

Every answer must remain traceable to the repository.

---

### Failure Modes

Query execution may fail due to:

- missing entities
- insufficient evidence
- invalid queries
- repository changes during execution

When evidence cannot be established, the query should return an explicit
"unknown" result rather than speculate.

---

## Incremental Processing

Incremental execution is the default operating mode.

Rather than rebuilding the entire repository model, the pipeline processes
only affected regions.

```mermaid
flowchart LR

RepositoryChange

--> DetectChanges

--> ReparseChangedFiles

--> ExtractFacts

--> UpdateKnowledgeGraph

--> Validate

--> PersistRevision

--> RecomputeAnalysis
```

Incremental processing minimizes unnecessary work while preserving
deterministic results.

---

## Glossary

This document references the following terms defined in [GLOSSARY.md](GLOSSARY.md):

- [Repository Snapshot](GLOSSARY.md#repository-snapshot)
- [Syntax Tree](GLOSSARY.md#syntax-tree)
- [Repository Fact](GLOSSARY.md#repository-fact)
- [Knowledge Graph](GLOSSARY.md#knowledge-graph)
- [Graph Revision](GLOSSARY.md#graph-revision)
- [Derived Fact](GLOSSARY.md#derived-fact)
- [Query Result](GLOSSARY.md#query-result)

---

## Pipeline Ownership

Each stage belongs to exactly one architectural subsystem.

| Pipeline Stage | Owning Subsystem |
|---------------|------------------|
| Repository Discovery | Acquisition (see [ACQUISITION.md](ACQUISITION.md)) |
| Parsing | Acquisition (see [ACQUISITION.md](ACQUISITION.md)) |
| Fact Extraction | Acquisition (see [ACQUISITION.md](ACQUISITION.md)) |
| Graph Construction | Knowledge Graph |
| Graph Validation | Knowledge Graph |
| Persistence | Storage |
| Analysis | Analysis |
| Query Execution | Query Engine |

Each subsystem owns one responsibility.

Ownership never overlaps.

---

## Error Handling

The pipeline is fail-fast.

A stage either:

- produces a valid artifact, or
- reports failure.

Partial artifacts are never consumed by downstream stages.

```mermaid
flowchart LR

Input

--> Stage

Stage --> Success

Stage --> Failure

Failure --> Abort

Success --> NextStage
```

This guarantees downstream stages always operate on valid inputs.

---

## Extension Points

The pipeline is intentionally extensible.

Supported extension points include:

- repository discovery
- language parsers
- manifest parsers
- fact extractors
- graph builders
- validators
- analysis algorithms
- exporters
- query providers

Extensions should integrate with existing stages rather than introducing new
processing paths.

---

## Pipeline Invariants

Every implementation of the repository processing pipeline must satisfy the
following constraints.

- deterministic
- incremental
- immutable artifacts
- evidence preservation
- reproducible outputs
- language independence
- fail-fast execution
- single responsibility per stage

These are architectural invariants.

No feature should violate them.

---

## Future Evolution

The pipeline is expected to grow over time.

Future processing stages may include:

- Git history analysis
- repository ownership analysis
- code generation tracking
- runtime trace integration
- test coverage ingestion
- security scanning
- performance profiling

These capabilities should extend existing artifacts rather than replacing
them.

The overall processing lifecycle remains unchanged:

```mermaid
flowchart LR

Repository

--> RepositorySnapshot

--> SyntaxTrees

--> RepositoryFacts

--> KnowledgeGraph["Knowledge Graph"]

--> ValidatedGraph["Validated Graph"]

--> GraphRevision["Graph Revision"]

--> DerivedFacts["Derived Facts"]

--> EvidenceBackedResults["Evidence-backed Results"]
```

The pipeline is intentionally designed so that new capabilities compose with
existing stages while preserving deterministic repository understanding.

---

## See Also

- [DESIGN.md](../DESIGN.md) — System design and invariants
- [Documentation index](README.md) — All documents

