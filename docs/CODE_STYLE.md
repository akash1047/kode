# Code Commenting Guidelines

## Philosophy

Code should explain **what**.

Comments should explain **why**.

Comments are part of the architecture, not decoration.

A useful comment reduces the amount of reasoning required to understand the code. A useless comment increases it.

Every comment should earn its place.

---

# Goals

Comments should:

- Explain architectural intent.
- Document invariants.
- Clarify ownership.
- Explain non-obvious design decisions.
- Describe subsystem boundaries.
- Prevent incorrect future refactoring.

Comments should **not**:

- Narrate obvious Rust syntax.
- Repeat type names.
- Describe individual statements.
- Become stale documentation.

---

# Comment Hierarchy

The project uses four levels of comments.

## 1. Module Documentation (Required)

Every public module must begin with module-level documentation.

It should answer:

- Why does this module exist?
- What does it own?
- What does it intentionally not do?

Example:

```rust
//! Stage 2: Parsing
//!
//! Converts immutable repository artifacts into immutable syntax artifacts.
//!
//! Responsibilities:
//! - parser dispatch
//! - syntax tree construction
//! - parse diagnostics
//!
//! Does not:
//! - perform semantic analysis
//! - interpret repository structure
//! - mutate pipeline artifacts
```

---

## 2. Public API Documentation (Required)

Every public:

- module
- struct
- enum
- trait
- public function

must have documentation.

Documentation should describe:

- purpose
- responsibility
- guarantees
- important invariants

Do **not** document obvious getters and setters.

Example:

```rust
/// Immutable repository-wide collection of parsing outcomes.
///
/// Every RepositoryFile produces exactly one FileParseOutcome,
/// even when parsing is skipped or fails.
pub struct SyntaxTreeInventory {
    ...
}
```

---

## 3. Architectural Comments (Required)

Architectural comments explain **why** something exists.

Examples:

```rust
// Preserve snapshot ordering so downstream stages remain deterministic.
```

```rust
// Tree-sitter remains encapsulated so later pipeline stages
// never depend on parser implementation details.
```

```rust
// Parser lookup is performed by language because Acquisition
// has already established the file's language.
```

These comments capture architectural intent that is not obvious from the implementation alone.

---

## 4. Local Implementation Comments (Rare)

Use implementation comments only when the code would otherwise require significant reasoning.

Example:

```rust
// Index mirrors `outcomes` and enables O(1) lookup while preserving
// deterministic iteration order.
```

---

# Required Comment Situations

Comments are expected whenever code introduces one of the following.

## Architectural Boundaries

Document subsystem boundaries.

```rust
// Parsing consumes immutable inputs only.
// Filesystem access occurs before this stage.
```

---

## Ownership Decisions

Explain ownership and lifetime choices.

```rust
// Source text is shared using Arc<str> to avoid duplication
// across multiple SyntaxTree instances.
```

---

## Invariants

Document assumptions that must always remain true.

```rust
// Every RepositoryFile produces exactly one outcome.
```

---

## Ordering Guarantees

Document deterministic behavior.

```rust
// Registration order determines parser precedence.
```

---

## Performance Decisions

Document intentional optimizations.

```rust
// Preallocate to avoid repeated allocations for large repositories.
```

---

## Error Handling Strategy

Document recovery behavior.

```rust
// Individual parse failures never abort repository parsing.
```

---

## Future Extension Points

Explain where new functionality belongs.

```rust
// Additional parser backends should integrate here rather than
// exposing backend-specific types throughout the subsystem.
```

---

# Avoid These Comments

Never comment obvious code.

Bad:

```rust
// Create parser.
let parser = Parser::new();
```

Bad:

```rust
// Return inventory.
inventory
```

Bad:

```rust
// Loop through files.
for file in files {
    ...
}
```

The code already communicates these actions.

---

# Prefer Stable Comments

Comments should describe architectural intent rather than implementation details.

Bad:

```rust
// Uses HashMap.
```

Good:

```rust
// Provides O(1) lookup while preserving deterministic iteration.
```

The implementation may change.

The architectural intent should not.

---

# Comment Density

This project intentionally favors **sparse, high-value comments**.

General guidance:

- Every public module: documentation.
- Every public API: documentation.
- Architectural comments where reasoning is not obvious.
- Roughly one meaningful implementation comment every 20–40 lines **only when justified**.
- Zero comments for self-explanatory code.

If removing a comment does not make the code harder to understand, the comment probably should not exist.

---

# Documentation vs Comments

Use Rust documentation comments (`//!`, `///`) for:

- public APIs
- architectural contracts
- stable behavior
- invariants

Use implementation comments (`//`) for:

- rationale
- ownership
- design decisions
- performance reasoning
- implementation constraints

Never duplicate the same information in both.

---

# Examples

Good:

```rust
// Parsing never reads from disk.
// Source acquisition occurs before this stage begins.
pub fn run(...)
```

Good:

```rust
// Unsupported languages remain visible in the pipeline so later
// stages can distinguish "not parsed" from "missing".
```

Good:

```rust
// The index exists purely for lookup efficiency.
// Iteration order is defined exclusively by `outcomes`.
```

Bad:

```rust
// Parse the file.
```

Bad:

```rust
// Get language.
```

Bad:

```rust
// Return true if empty.
```

---

# Review Checklist

Before committing code, verify:

- [ ] Every public module has module documentation.
- [ ] Every public type has documentation.
- [ ] Every public trait has documentation.
- [ ] Every public function has documentation.
- [ ] Architectural decisions are explained where appropriate.
- [ ] Ownership and lifetime decisions are documented.
- [ ] Important invariants are documented.
- [ ] Performance-sensitive code explains *why* the optimization exists.
- [ ] Extension points are clearly identified.
- [ ] No comments merely restate the code.
- [ ] Comments remain accurate after refactoring.

---

# Guiding Principle

When writing a comment, ask:

> **Will this comment help a contributor understand the architectural intent, invariants, ownership, or reasoning behind the code six months from now?**

If the answer is **yes**, keep it.

If the answer is **no**, let the code speak for itself.
