# CLI

> **Implementation status:** The CLI is currently a placeholder binary (`tools/cli/src/main.rs` contains only `fn main() {}`). The command hierarchy and examples below describe the target design.

This document describes the **kode** command-line interface.

---

## Command Hierarchy

```
kode
├── (no subcommand)        # Scan current repository
├── config
│   ├── init               # Initialize configuration
│   └── set <key> <value>  # Set configuration value
├── chat
│   ├── (no args)          # Start interactive chat session
│   └── -m <message>       # Ask a single question and exit
├── mcp serve <path>       # Start MCP server for a repository
└── cache
    ├── status             # Inspect repository cache
    └── clear              # Remove repository cache
```

---

## Global Options

| Option | Description |
|--------|-------------|
| `--help` | Display help information |
| `--version` | Display version information |

---

## Configuration

Configuration is managed through `kode config` commands.

```sh
kode config init
kode config set <key> <value>
```

---

## Examples

**Scan the current repository:**
```sh
kode
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

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `KODE_CACHE_DIR` | Custom cache directory path |

---

## Output Conventions

- All answers include `path:line` citations
- Evidence is always verifiable against the live repository
- Command output follows a consistent format for machine parsing where practical

---

## Common Workflows

**First-time setup:**
```sh
kode config init
kode
```

**Daily use:**
```sh
kode
kode chat
```

**CI pipeline integration:**
```sh
kode --json
```

---

## See Also

- [DESIGN.md](../DESIGN.md) — System design
- [MCP.md](MCP.md) — MCP server
- [Documentation index](README.md)
