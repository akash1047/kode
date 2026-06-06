# Contributing to kode

Please read and follow our [Code of Conduct](CODE_OF_CONDUCT.md).

## Prerequisites

- Rust stable toolchain (`rustup install stable`)
- `cargo` (included with Rust)

## Build

```sh
cargo build --release
# binary at target/release/kode
```

## Test

```sh
cargo test --all
```

## Lint

```sh
cargo fmt --check
cargo clippy -- -D warnings
```

Fix formatting in-place with `cargo fmt`.

## Design contract

Before contributing features or changes, read [DESIGN.md](DESIGN.md). It defines what kode is, what it is not, and the cache invariants that keep answers honest. Changes that violate those invariants will be rejected.

## Commit style

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add Ruby symbol extraction
fix: revalidate cache on mtime mismatch
docs: expand CLI reference for mcp serve
chore: bump tree-sitter to 0.24
```

Subject line ≤ 72 characters. Body optional — use it when the *why* is non-obvious.

## Pull request checklist

- [ ] `cargo test --all` passes
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] New behaviour is covered by tests or documented in docs/
- [ ] `CHANGELOG.md` updated under `[Unreleased]`

## CI

All PRs must pass GitHub Actions checks (test, fmt, clippy) before merge.

## Reporting issues

Use the GitHub issue templates. For security vulnerabilities, see [SECURITY.md](SECURITY.md).
