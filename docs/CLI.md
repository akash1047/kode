# CLI

This document describes the **kode** command-line interface.

## Implementation Status

The CLI application is built on a two-layer architecture:

1. **Application layer** (`kode-app` service) — owns the scan pipeline lifecycle: repository discovery, source loading, parsing, fact extraction, graph construction, and persistence. Exposes `run_scan()`, `ScanResult`, and `ScanStatistics`.
2. **Presenter/Formatter layer** (`kode-cli` tools) — presenter structs transform domain types into display models; formatter functions render those models as text or JSON.

**All listed subcommands are implemented** and wired to the application / query / agent / MCP layers:

| Command | Status |
|---------|--------|
| `scan` | Full pipeline; fingerprint-based incremental skip when unchanged |
| `status` | Cache-backed graph stats |
| `files` | Files from knowledge graph |
| `symbols` | Entity listing with optional language filter |
| `query` | Intent router: search, callers, callees, impact, metrics, dead, cycles |
| `export` | DOT / GraphML |
| `chat` | Agent TUI + one-shot `-m` |
| `cache` | status / clear |
| `config` | init / get / set |
| `mcp serve` | MCP server (8 tools) |

Flags not yet fully implemented: `--watch` (exits with error), `--threads` (accepted, unused).

---

## Command Hierarchy

```
kode
├── scan [PATH]            # Discover and index a repository
├── status                 # Show repository indexing status
├── files                  # Explore indexed repository files
├── symbols                # Explore extracted symbols
├── query <QUERY>          # Query repository knowledge
├── export                 # Export graph (dot | graphml)
├── chat
│   ├── (no args)          # Start interactive chat session
│   └── -m <message>       # Ask a single question and exit
├── cache
│   ├── status             # Show cache information
│   └── clear              # Remove cached repository data
├── config
│   ├── init               # Create configuration
│   ├── get <key>          # Read a configuration value
│   └── set <key> <value>  # Update a configuration value
├── mcp
│   └── serve <path>       # Start the MCP server
└── help                   # Print help for a command
```

---

## Global Options

| Option | Description |
|--------|-------------|
| `-C, --repo <PATH>` | Repository to operate on |
| `-v, --verbose` | Increase logging verbosity (repeatable) |
| `-q, --quiet` | Suppress non-essential output |
| `--json` | Machine-readable output |
| `--no-color` | Disable colored output |
| `--log-file <PATH>` | Write logs to file (default: .kode/logs/kode.log) |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

Global options are accepted by all subcommands.

---

## Subcommands

### scan

Discover repository structure, parse supported languages, extract facts, build the knowledge graph, and persist a revision under `.kode/cache.db`.

When the content fingerprint (paths + sizes + mtimes) matches the last stored revision, stages 2–5 are **skipped** (`cache hit`). Use `--full` to clear the cache and rebuild.

```sh
kode scan [PATH] [OPTIONS]
```

Options:
- `--full` — Clear cache and rebuild from scratch
- `--watch` — Not implemented yet (errors)
- `--threads <N>` — Accepted but unused (scan is single-threaded)

### status

Show repository index status from the local cache (does not re-scan).

```sh
kode status
```

### files

List files known to the knowledge graph with optional language filtering.

```sh
kode files [OPTIONS]
```

### symbols

List extracted language symbols (functions, types, traits, modules, etc.).

```sh
kode symbols [OPTIONS]
```

### query

Deterministic queries against the knowledge graph. Intent is resolved by prefix:

| Query | Meaning |
|-------|---------|
| `name` / prefix | Symbol search |
| `callers:NAME` | Incoming call edges |
| `callees:NAME` | Outgoing call edges |
| `impact:NAME` | Reverse-call impact BFS (default depth 8) |
| `impact:NAME:N` | Impact with max depth N |
| `metrics` / `metrics:NAME` | Fan-in / fan-out / instability |
| `dead` | Heuristic unreferenced functions |
| `cycles` | Call-graph SCCs (cycles) |

```sh
kode query "run_scan"
kode query "callers:run_scan"
kode query "metrics"
kode query "dead"
kode query "cycles"
```

### export

Export the indexed knowledge graph.

```sh
kode export --format dot
kode export --format graphml -o graph.xml
```

### chat

Interactive repository assistant (agent TUI) with sandboxed FS tools and symbol/call-graph tools when an index exists.

```sh
kode chat
kode chat -m "Where is run_scan defined?"
```

Configure via `.kode/config.toml` (`chat.provider`, `chat.model`, `chat.api_key`, `chat.api_base`) or env (`OPENAI_API_KEY`, `OLLAMA_API_KEY`, `OPENAI_BASE_URL`).

### cache / config / mcp

Implemented as above. MCP tools: `find_symbol`, `search_symbols`, `get_symbol_details`, `symbols_by_kind`, `read_file`, `find_callers`, `find_callees`, `impact_analysis`.

---

## Examples

```sh
kode config init
kode scan
kode scan          # second run: cache hit if nothing changed
kode status
kode files --language rust
kode symbols
kode query "impact:run_scan"
kode export --format dot | head
kode mcp serve .
```

---

## See Also

- [CLI_SPEC.md](CLI_SPEC.md) — clap help contract
- [ROADMAP.md](ROADMAP.md) — milestones
- [CHAT_UI.md](CHAT_UI.md) — chat agent design
