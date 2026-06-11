# kode CLI Reference

## Commands

```sh
kode [PATH] [COMMAND]
```

All subcommands accept an optional `PATH` (defaults to `.`).

### Top-level

| Command | What it does |
|---------|-------------|
| `kode [PATH]` | Project header + per-file char counts |
| `kode info [PATH]` | Project header only |
| `kode scan [PATH]` | Project header + per-file char counts |

### chat

Open an interactive chat REPL backed by the cache and project tools.

```sh
kode chat [PATH]                    # interactive REPL
kode chat [PATH] -m "question"      # one-shot: print answer and exit
kode chat [PATH] --model MODEL      # override model for this invocation
```

#### REPL slash commands

| Command | What it does |
|---------|-------------|
| `/help` | Print available commands |
| `/reset` | Start a new conversation (clears session) |
| `/model [NAME]` | Show current model, or switch to NAME |
| `/clear` | Clear the terminal |
| `/exit` | Quit |

#### Tools available to the model

| Tool | What it does |
|------|-------------|
| `list_files` | Enumerate project files (gitignore-aware). Use for filename/path discovery. |
| `find_symbol` | Locate a definition by name in the symbol index (Rust, Python, TypeScript, JavaScript, Go, Java, C, C++, C#, Ruby). |
| `search_project` | Regex search over file contents (not filenames). |
| `read_project_files` | Read source spans (batched). Truth source — always read before citing. |

### cache

Build, inspect, and clear the project cache.

```sh
kode cache build [PATH]     # build/refresh cache without opening chat
kode cache status [PATH]    # show cache stats (db path, counts, last scan time)
kode cache clear [PATH]     # delete the cache for a repo
kode cache inspect [PATH]   # human-friendly breakdown: files by language, symbols by kind, manifests, run-configs
```

### mcp

Expose the kode index to other agents over the Model Context Protocol.

```sh
kode mcp init [PATH]                                # write editor MCP config (skips if exists)
kode mcp serve [PATH]                               # stdio transport (default)
kode mcp serve [PATH] --transport http --port 8765  # HTTP transport
```

#### mcp init

Writes an MCP config file into the project directory for the chosen editor preset. Exits with an error if the file already exists.

| Flag | Default | Description |
|------|---------|-------------|
| `--preset` | `claude` | Editor preset: `claude`, `vscode`, `cursor`, `zed` |

| Preset | File written |
|--------|-------------|
| `claude` | `.mcp.json` |
| `vscode` | `.vscode/mcp.json` |
| `cursor` | `.cursor/mcp.json` |
| `zed` | `.zed/settings.json` |

```sh
kode mcp init                        # writes .mcp.json for Claude Code
kode mcp init --preset vscode        # writes .vscode/mcp.json
kode mcp init --preset cursor        # writes .cursor/mcp.json
kode mcp init --preset zed           # writes .zed/settings.json
```

#### mcp serve

Flags:

| Flag | Default | Description |
|------|---------|-------------|
| `--transport` | `stdio` | Transport: `stdio` or `http` |
| `--port` | `8765` | Port for HTTP transport |

Claude Desktop config (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "kode": {
      "command": "/path/to/kode",
      "args": ["mcp", "serve", "/path/to/project"]
    }
  }
}
```

---

## Path restrictions

kode refuses to scan certain paths and exits with an error:

| Path | Reason |
|------|--------|
| `/` (filesystem root) | Would walk the entire system |
| `~` (home directory) | Too broad; pass a specific project subdirectory |

Subdirectories under home (e.g. `~/projects/myapp`) are allowed.

### config

Manage user configuration at `~/.config/kode/config.toml`.

```sh
kode config init                    # write default template (skips if exists)
kode config show                    # print config (api_key redacted)
kode config set <key> <value>       # set a key (dot-separated for section, bare for [default])
kode config get <key>               # print bare value of a key
```

Keys are dot-separated (`section.field`). Bare keys (no dot) target `[default]` automatically:

```sh
kode config set api_key sk-xxx          # same as default.api_key
kode config set model llama3:70b        # same as default.model
kode config set chat.model llama3:70b   # chat section only
```

Examples:

```sh
kode config init
kode config set api_key sk-xxx
kode config set chat.model llama3:70b
kode config set summarize.max_input_chars 4000
kode config get chat.model
kode config show
```

---

## Configuration file

Location: `~/.config/kode/config.toml`

```toml
[default]
api_key  = ""                        # shared fallback for all subsystems
model    = "nemotron-3-nano:30b"
base_url = "https://ollama.com/v1"

[chat]
api_key  = ""                        # overrides [default].api_key
model    = ""                        # overrides [default].model
base_url = ""                        # overrides [default].base_url

[summarize]
api_key         = ""                 # overrides [default].api_key
model           = ""                 # overrides [default].model
base_url        = ""                 # overrides [default].base_url
max_input_chars = 8000               # truncate files before sending to model
```

### Supported keys

| Key | Type | Description |
|-----|------|-------------|
| `default.api_key` | string | API key for all subsystems |
| `default.model` | string | Model name for all subsystems |
| `default.base_url` | string | API base URL for all subsystems |
| `chat.api_key` | string | API key for chat (overrides default) |
| `chat.model` | string | Model for chat (overrides default) |
| `chat.base_url` | string | Base URL for chat (overrides default) |
| `summarize.api_key` | string | API key for summarization (overrides default) |
| `summarize.model` | string | Model for summarization (overrides default) |
| `summarize.base_url` | string | Base URL for summarization (overrides default) |
| `summarize.max_input_chars` | integer | Max chars sent per file to summarize model |

---

## Environment variables

| Variable | Overrides |
|----------|-----------|
| `KODE_MODEL_API_KEY` | `[chat].api_key` and `[summarize].api_key` |
| `KODE_MODEL` | `[chat].model` |
| `KODE_SUMMARIZE_MODEL` | `[summarize].model` |
| `KODE_SUMMARIZE_BASE_URL` | `[summarize].base_url` |

---

## Resolution order

Per field, highest to lowest priority:

```
CLI flag (--model)
  ↓
Environment variable
  ↓
[chat] / [summarize] config section
  ↓
[default] config section
  ↓
Built-in default
```

`api_key` has no built-in default — it errors if unset at all levels.

---

## Loading API key from a file

Fish:

```fish
env KODE_MODEL_API_KEY=(string trim < ~/.secrets/ollama) kode chat
```

Bash:

```sh
KODE_MODEL_API_KEY="$(tr -d '[:space:]' < ~/.secrets/ollama)" kode chat
```

Or set it permanently in the config:

```sh
kode config set api_key "$(tr -d '[:space:]' < ~/.secrets/ollama)"
```
