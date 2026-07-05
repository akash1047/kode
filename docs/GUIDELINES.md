# Workspace Guidelines

## Workspace Name

**kode**

A modular Rust workspace where components can be developed, tested, and
versioned independently while being composed into one or more applications.

---

## Directory Structure

```
kode/
├── Cargo.toml              # Workspace manifest
├── Cargo.lock
├── rust-toolchain.toml     # Toolchain channel & edition
├── README.md
│
├── crates/                 # Reusable library crates
├── services/               # Long-running applications
├── tools/                  # CLI utilities
├── examples/
├── docs/
└── scripts/
```

---

## Library Crates (`crates/`)

Crate boundaries follow the architectural subsystems defined in `DESIGN.md`
and `docs/ARCHITECTURE.md`. Each library crate corresponds to one subsystem.

### Must

- Have a single, well-defined responsibility matching its subsystem.
- Be independently testable.
- Minimize dependencies on other workspace crates.
- Avoid application startup or runtime orchestration.

### Must Not

- Parse CLI arguments.
- Read environment variables directly.
- Start servers or background workers.
- Contain deployment-specific logic.

---

## Binary Crates (`services/` and `tools/`)

Binary crates compose subsystem crates into runnable applications.

### Must

- Contain the application entry point (`main.rs`).
- Wire together library crates.
- Load configuration and initialize infrastructure.
- Handle process lifecycle and graceful shutdown.

Binary crates should remain **thin**, delegating all logic to library crates.

---

## Dependency Management

- Use a Cargo workspace.
- Define shared dependency versions in `[workspace.dependencies]` in the
  root `Cargo.toml`.
- Inherit shared dependencies using `*.workspace = true`.
- Use path dependencies for internal workspace crates.
- Keep dependency versions centralized — never hard-code a version in a
  member crate if it can be shared.

---

## Dependency Direction

Dependencies must flow in **one direction only**:

```
Applications (services/, tools/)
      │
Subsystem Crates (crates/)
```

Subsystem crate dependencies follow the data flow defined in the architecture:

- `acquisition` → `graph`
- `graph` → `storage`
- `graph` → `analysis`
- `analysis` / `graph` → `query`
- `common` is the lowest layer — no workspace dependencies

No circular dependencies are permitted.

---

## Crate Design

- Each crate must have a **clear public API**.
- Hide implementation details behind the public surface.
- Minimise the exposed surface area — `pub` only what consumers need.
- Prefer **traits** for extensibility over concrete types.
- Avoid unnecessary coupling between crates.
- Keep crates **focused on a single responsibility**.

---

## Testing

Each crate owns its tests.

- **Unit tests** live inline in the crate's source.
- **Integration tests** live in `tests/` at the crate level or workspace
  root.
- **End-to-end tests** live in `tests/` at the workspace root.

The entire workspace can be tested with:

```bash
cargo test --workspace
```

---

## Tooling

The workspace supports these commands without errors or warnings:

```bash
cargo check --workspace
cargo build --workspace
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets
```

These commands are enforced by CI. The exact CI equivalents use stricter flags:

```bash
cargo fmt --all -- --check       # Fail on formatting differences
cargo clippy --workspace --all-targets -- -D warnings  # Deny warnings
```


## Continuous Integration

The project uses GitHub Actions for CI. The workflow is defined in
`.github/workflows/ci.yml`.

### What CI validates

| Step | Command |
|------|---------|
| Formatting | `cargo fmt --all -- --check` |
| Linting | `cargo clippy --workspace --all-targets -- -D warnings` |
| Compilation | `cargo build --workspace` |
| Tests | `cargo test --workspace` |
| Documentation | `cargo doc --workspace --no-deps` |

### When CI runs

- On every push to `main` and `dev*` branches.
- On every pull request targeting `main` and `dev*`.

### Reproducing individual checks

Run the same commands that CI executes. Each command validates a single
aspect of the workspace:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo test --workspace
cargo doc --workspace --no-deps
```

All commands must succeed. CI uses the exact same commands.

### Running the complete CI workflow

The repository supports local CI validation using [`act`](https://github.com/nektos/act).
Docker is required. Run the workflow locally before pushing to catch
configuration and environment issues early.

#### Listing available jobs

```bash
act -l
```

Lists the available jobs defined in the workflow.

#### Running validation

```bash
act -j validate
```

Executes the `validate` job from `.github/workflows/ci.yml`. This is
the recommended way to validate CI changes locally.

#### Repository configuration

The repository includes `.actrc` which pins the runner image used by `act`.
Contributors normally do not need to modify it.

#### Known limitations

The `actions/upload-artifact` step fails under a default `act` setup:

```text
Unable to get the ACTIONS_RUNTIME_TOKEN env variable
```

This is expected — `act` does not implement the GitHub Artifact service.
The step is harmless; all other CI steps complete successfully. GitHub
Actions uploads artifacts correctly.

### Pull request expectations

Before opening a pull request:

- All CI checks pass on your branch.
- No Clippy warnings are introduced.
- Documentation builds without errors.
- Existing tests continue to pass.
- New features include tests.

---

## Documentation

The kode documentation set is organized into two categories:

- **Architecture documents** — system design, pipeline, Knowledge Graph, storage, query engine, analysis, evidence model, and MCP. See [README.md](README.md) for the full index.
- **Contributor documents** — workspace guidelines, crate overview, CLI reference, glossary, and roadmap.

### Key References

- [Documentation index](README.md) — All documents
- [CRATE_OVERVIEW.md](CRATE_OVERVIEW.md) — Crate responsibilities and dependency graph
- [ARCHITECTURE.md](ARCHITECTURE.md) — High-level architecture
- [DESIGN.md](../DESIGN.md) — System design principles

---

## Design Principles

| Principle | Description |
|---|---|
| **Composition over inheritance** | Build behaviour by combining small, focused pieces. |
| **Single responsibility** | Each crate has one well-defined purpose. |
| **Separation of concerns** | Reusable logic is separate from application wiring. |
| **Thin binaries** | Binaries wire up libraries; they do not contain business logic. |
| **No global state** | Avoid global state where possible. |
| **Independent reusability** | Library crates must be usable outside this workspace with minimal friction. |
| **Explicit interfaces** | Favour traits and public APIs over hidden behaviour. |
| **Maintainability over convenience** | Optimise for long-term clarity, not short-term ease. |

---

## Acceptance Criteria Checklist

- [ ] Workspace builds successfully (`cargo build --workspace`).
- [ ] Components can be developed independently.
- [ ] Multiple applications can reuse the same libraries.
- [ ] Shared dependencies are centrally managed in `[workspace.dependencies]`.
- [ ] No circular dependencies exist.
- [ ] Formatting, linting, and tests pass across the workspace.
- [ ] Workspace structure and conventions are documented.
