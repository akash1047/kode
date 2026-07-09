# kode

Rust workspace of 8 crates. "Evidence-first code intelligence" -- parse repos, extract facts, build knowledge graph, store in SQLite.

## Commands

```bash
cargo check --workspace        # fast verify
cargo build --workspace        # full build
cargo test --workspace         # all tests
cargo fmt --all -- --check     # style gate
cargo clippy --workspace --all-targets -- -D warnings  # lint gate (CI enforces)
cargo doc --workspace --no-deps
```

CI runs: fmt → clippy → build → test → doc. All must pass.

## Workspace layout

```
crates/
  common/         # no deps. FNV-1a hasher for deterministic IDs. lowest crate.
  acquisition/    # stage 1: discover repos, detect lang/workspace, walk FS
  analysis/       # stage 2-3: tree-sitter parse + fact extraction. knows Rust only.
  graph/          # stage 4: knowledge graph builder + validator. deterministic.
  storage/        # stage 5: SQLite (bundled) persistence. rusqlite + serde_json.
  query/          # placeholder. no code.
services/app/     # pipeline orchestrator: run_scan() -> ScanResult
tools/cli/        # binary. clap 4 derive. subcommands: scan, status, files, symbols, query, chat, cache, config, mcp
```

Dependency flow (enforced): `acq -> graph -> storage`, `graph -> analysis`, `analysis/graph -> query`, `common` lowest.

## Hard constraints

- `unsafe_code = "deny"` workspace-wide. zero unsafe.
- `clippy.all = "warn"` with priority -1. `-D warnings` in CI.
- No circular dependencies.
- All IDs (EntityId, StructuralNodeId) are deterministic FNV-1a u64 hashes from `kode-common`.
- Every graph element must have repository evidence.

## Lint overrides (know these exist)

- `crates/analysis/src/lib.rs`, `crates/graph/src/lib.rs`, `crates/storage/src/lib.rs`: `#![allow(unused_crate_dependencies)]`
- `crates/acquisition/tests/discovery.rs`: same

## Suppress-pattern for workspace dep lints

```rust
use thiserror as _;   // in lib.rs when crate is used only by re-export
use tempfile as _;    // when crate is dev-dep only
```

## Toolchain

stable, edition 2021. rust-toolchain.toml is source of truth (1.75 MSRV, 1.85 README badge).

## Style

Comment hierarchy: `//!` module docs (required), `///` public API docs (required), `//` why-comments, `//` local impl (rare). Architecture docs in `docs/` follow Purpose→Position→Architecture→Implementation→Future template.

## Testing

- `tempfile::TempDir` for FS tests. `dummy_repo()` helper returns `Repository::new(".")`.
- Builder pattern (`SnapshotBuilder`) for test data.
- CLI has 38 inline tests in `main.rs`. Integration tests in `tests/` dirs.
- No snapshot testing framework. Determinism checked via manual round-trip.

## Gotchas

- `Cargo.lock` is in `.gitignore` (library convention). Not committed.
- `status` subcommand re-scans instead of reading cache.
- `scripts/` and `examples/` exist but are empty (`.gitkeep`).
- Only `tree-sitter-rust` grammar bundled. Other languages: Error.
- `SyntaxBackend` (tree-sitter wrapper) is `pub(crate)`.
- SQLite schema version 1.0, checked at init, no migration system yet.
- Zoekt index at `.zoekt/` for code search.
