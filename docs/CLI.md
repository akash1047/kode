# CLI

This document describes the **kode** command-line interface.

## Implementation Status

The CLI foundation is complete. The acquisition library crate provides:

- Repository discovery
- Workspace detection
- Filesystem traversal
- Manifest discovery
- Language detection
- RepositorySnapshot construction

The current implementation does **not** yet provide:

- Source code parsing or symbol extraction
- Knowledge graph construction or querying
- Storage, caching, or incremental updates
- MCP server logic or LLM integration

All subcommands currently accept and validate their arguments, then dispatch to a placeholder handler. The `scan` subcommand has not yet been wired to the acquisition library. Backend functionality described below documents the **intended purpose** of each command once the repository processing pipeline is wired to the CLI.

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
| `-h, --help` | Print help |
| `-V, --version` | Print version |

Global options are accepted by all subcommands.

---

## Subcommands

### scan

Discover and index a repository (placeholder — not yet wired to the acquisition library).

The intended pipeline is:

```
Repository → RepositoryDiscovery → RepositorySnapshot
```

Once wired, this will produce an immutable structural snapshot of the repository (workspace layout, files, directories, manifests, and detected languages). Parsing and downstream stages remain future work.

```sh
kode scan [PATH] [OPTIONS]
```

Options:
- `--full` — Ignore incremental state and rebuild from scratch
- `--watch` — Monitor repository for filesystem changes
- `--threads <N>` — Worker threads

### status

Show repository indexing status. Once indexing is implemented, this will display repository metadata, indexed file statistics, language breakdowns, graph information, and cache location. Currently it validates arguments and dispatches to a placeholder handler.

```sh
kode status
```

### files

Explore repository files. Once the knowledge graph is implemented, this will list files known to the graph with optional filtering by language, modified status, or ignored status. Currently it validates arguments and dispatches to a placeholder handler.

```sh
kode files [OPTIONS]
```

Options:
- `--language <LANG>` — Filter by language
- `--modified` — Show only modified files
- `--ignored` — Show ignored files

### symbols

Explore extracted language symbols (functions, types, traits, classes, etc.). Once parsing is implemented, this will display symbols extracted from indexed source files. Currently it validates arguments and dispatches to a placeholder handler.

```sh
kode symbols [OPTIONS]
```

Options:
- `--language <LANG>` — Filter by language

### query

Query repository knowledge. Once the query engine is implemented, this will execute deterministic queries against the knowledge graph. Currently it validates arguments and dispatches to a placeholder handler.

```sh
kode query "<query>"
```

### chat

Start an interactive repository assistant session. Once LLM integration is implemented, this will answer repository questions using live source code and evidence-backed citations. Currently it validates arguments and dispatches to a placeholder handler.

```sh
kode chat [OPTIONS]
```

Options:
- `-m, --message <TEXT>` — Ask one question and exit

### cache

Manage the local repository cache.

```sh
kode cache <COMMAND>
```

Commands:
- `status` — Show cache information
- `clear` — Remove cached repository data

### config

Manage kode configuration.

```sh
kode config <COMMAND>
```

Commands:
- `init` — Create configuration
- `get <key>` — Read a configuration value
- `set <key> <value>` — Update a configuration value

### mcp

Run or manage the MCP server.

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

**First-time setup:**
```sh
kode config init
kode scan
```

**Daily use:**
```sh
kode scan
kode chat
```

**CI pipeline integration:**
```sh
kode status --json
```

---

## See Also

- [DESIGN.md](../DESIGN.md) — System design
- [MCP.md](MCP.md) — MCP server
- [Documentation index](README.md)
