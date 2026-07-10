# CLI

This document describes the **kode** command-line interface.

## Implementation Status

The CLI application is built on a two-layer architecture:

1. **Application layer** (`kode-app` service) — owns the scan pipeline lifecycle: repository discovery, source loading, parsing, and statistics collection. Exposes `run_scan()`, `ScanResult`, and `ScanStatistics`.
2. **Presenter/Formatter layer** (`kode-cli` tools) — presenter structs (`ScanView`, `StatusView`, `FilesView`) transform domain types into display models; formatter functions render those models as text.

The `scan`, `status`, and `files` subcommands are wired to the application pipeline and produce real results. The remaining subcommands (`symbols`, `query`, `chat`, `cache`, `config`, `mcp`) accept and validate arguments but dispatch to a placeholder handler.

---

## Command Hierarchy

```
kode
├── scan [PATH]            # Discover and index a repository
├── status                 # Show repository indexing status
├── files                  # Explore indexed repository files
├── symbols                # Explore extracted symbols
├── query <QUERY>          # Query repository knowledge
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

Discover repository structure, load source files, and parse supported languages.

Executes the full scan pipeline via `kode_app::run_scan()`:
1. Repository discovery (workspace detection, filesystem traversal, manifest discovery, language detection)
2. Source inventory loading
3. Parsing with language-specific parsers (currently Rust via tree-sitter)

Produces a `RepositorySnapshot` and scan statistics (files discovered, parsed, skipped, recovered, failed).

```sh
kode scan [PATH] [OPTIONS]
```

Options:
- `--full` — Ignore incremental state and rebuild from scratch
- `--watch` — Monitor repository for filesystem changes
- `--threads <N>` — Worker threads

### status

Show repository scan results. Executes the scan pipeline and displays repository metadata, file statistics, language breakdown, and parse statistics (parsed, recovered, skipped, failed).

```sh
kode status
```

### files

List discovered files from the scan pipeline with optional language filtering.

```sh
kode files [OPTIONS]
```

Options:
- `--language <LANG>` — Filter by language
- `--modified` — Show only modified files
- `--ignored` — Show ignored files

### symbols

Explore extracted language symbols (functions, types, traits, classes, etc.). Planned — currently a placeholder handler.

```sh
kode symbols [OPTIONS]
```

Options:
- `--language <LANG>` — Filter by language

### query

Query repository knowledge. Planned — currently a placeholder handler.

```sh
kode query "<query>"
```

### chat

Start an interactive repository assistant session. Queries the LLM (OpenAI default) with code evidence from the knowledge graph.

```sh
kode chat [OPTIONS]
```

Options:
- `-m, --message <TEXT>` — Ask one question and exit

### cache

Manage the local repository cache. Planned — currently a placeholder handler.

```sh
kode cache <COMMAND>
```

Commands:
- `status` — Show cache information
- `clear` — Remove cached repository data

### config

Manage kode configuration. Planned — currently a placeholder handler.

```sh
kode config <COMMAND>
```

Commands:
- `init` — Create configuration
- `get <key>` — Read a configuration value
- `set <key> <value>` — Update a configuration value

### mcp

Run or manage the MCP server. Planned — currently a placeholder handler.

```sh
kode mcp <COMMAND>
```

Commands:
- `serve <path>` — Start the MCP server

All subcommands accept global options (see above).

---

## Examples

**Scan the current repository:**
```sh
kode scan
```

**Scan a specific repository:**
```sh
kode scan /path/to/repo --full
```

**Show repository status:**
```sh
kode status
```

**List indexed files by language:**
```sh
kode files --language rust
```

**List extracted symbols:**
```sh
kode symbols --language python
```

**Query repository knowledge:**
```sh
kode query "functions named parse"
```

**Start an interactive chat session:**
```sh
kode chat
```

**Ask a single question:**
```sh
kode chat -m "Where is authentication implemented?"
```

**Run the MCP server:**
```sh
kode mcp serve .
```

**Check cache status:**
```sh
kode cache status
```

**Clear the repository cache:**
```sh
kode cache clear
```

**Initialize configuration:**
```sh
kode config init
kode config set api_key YOUR_KEY
```

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `KODE_CACHE_DIR` | Custom cache directory path |

---

## Output Conventions

- Command output follows a consistent format for machine parsing where practical
- Once LLM integration is implemented, all answers will include `path:line` citations verified against the live repository

---

## Common Workflows

**First-time scan:**
```sh
kode scan
kode status
```

**Daily use:**
```sh
kode scan
kode files --language rust
```

---

## See Also

- [DESIGN.md](../DESIGN.md) — System design
- [MCP.md](MCP.md) — MCP server
- [Documentation index](README.md)
